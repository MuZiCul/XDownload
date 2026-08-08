//! Multi-task download queue.
//!
//! Independent from the single-download path (download page): tasks enqueued
//! here run concurrently up to the configured concurrency, each in its own
//! `download_cache/{task_id}/` directory. Retries, dedup and cancellation are
//! handled here; the underlying `YtDlpDownloader::download(Some(id), …)` runs
//! without the single-download mutex.

use crate::downloader::ytdlp::YtDlpDownloader;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::services::config::ConfigManager;
use crate::services::download_history::DownloadHistory;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

static TASK_SEQ: AtomicUsize = AtomicUsize::new(0);

/// Generate a unique task id: a plain increasing number. Also used as the
/// per-task download_cache/{id}/ directory name so concurrent/paused tasks
/// never share cache files.
fn gen_task_id() -> String {
    TASK_SEQ.fetch_add(1, Ordering::Relaxed).to_string()
}

/// Advance the id counter past the largest restored id (keeps ids unique
/// across restarts when persistence is enabled).
fn note_restored_id(id: &str) {
    if let Ok(n) = id.parse::<usize>() {
        let cur = TASK_SEQ.load(Ordering::Relaxed);
        if n + 1 > cur {
            TASK_SEQ.store(n + 1, Ordering::Relaxed);
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QueuedTask {
    pub id: String,
    /// Stable enqueue order (used by the frontend to show "current x/N").
    pub seq: u64,
    pub config: DownloadConfig,
    pub title: Option<String>,
    /// True when this task is being resumed after a pause — the downloader
    /// keeps the cache directory so yt-dlp can resume from the .part file.
    #[serde(default)]
    pub resume: bool,
    /// Frontend card metadata (thumbnail / uploader / duration / …), persisted
    /// with the task so resumed tasks keep their info after a restart.
    #[serde(default)]
    pub info: Option<serde_json::Value>,
    /// Persisted state used to restore the task after a restart:
    /// "queued" | "running" | "paused" (empty = legacy record → queued).
    #[serde(default)]
    pub status: String,
}

#[derive(Default)]
struct QueueState {
    /// Tasks waiting to start.
    queued: VecDeque<QueuedTask>,
    /// Tasks currently downloading.
    running: Vec<QueuedTask>,
    /// Tasks explicitly paused by the user (not started / interrupted).
    paused_tasks: VecDeque<QueuedTask>,
    /// When paused, no new tasks are started (running tasks finish).
    paused: bool,
}

#[derive(Clone)]
pub struct DownloadQueue {
    app: AppHandle,
    downloader: Arc<YtDlpDownloader>,
    state: Arc<Mutex<QueueState>>,
}

impl DownloadQueue {
    pub fn new(app: AppHandle, downloader: Arc<YtDlpDownloader>) -> Self {
        Self {
            app,
            downloader,
            state: Arc::new(Mutex::new(QueueState::default())),
        }
    }

    /// Enqueue a download. Returns the task id, or an error when the URL is
    /// already queued or running (automatic dedup).
    ///
    /// `auto_start`: when true the queue starts draining immediately (single
    /// task use case); when false the task only waits until `start()` is
    /// called (batch mode — the user presses "开始任务" to begin).
    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        &self,
        config: DownloadConfig,
        title: Option<String>,
        auto_start: bool,
        info: Option<serde_json::Value>,
    ) -> Result<String, String> {
        let url = config.url.trim().to_string();
        if url.is_empty() {
            return Err("链接不能为空".to_string());
        }
        let seq = TASK_SEQ.fetch_add(1, Ordering::Relaxed) as u64;
        let id = seq.to_string();
        {
            let mut st = self.state.lock().unwrap();
            let dup = st
                .queued
                .iter()
                .chain(st.running.iter())
                .chain(st.paused_tasks.iter())
                .any(|t| t.config.url == url);
            if dup {
                return Err("链接已在队列中".to_string());
            }
            st.queued.push_back(QueuedTask {
                id: id.clone(),
                seq,
                config,
                title: title.clone(),
                resume: false,
                info: info.clone(),
                status: "queued".to_string(),
            });
        }
        let _ = self.app.emit(
            "download-queued",
            serde_json::json!({ "task_id": id, "url": url, "title": title, "info": info }),
        );
        self.persist();
        if auto_start {
            self.pump();
        }
        Ok(id)
    }

    /// Start up to `concurrency` tasks. Called after enqueue and after a task
    /// finishes, so the queue keeps draining automatically.
    fn pump(&self) {
        let concurrency = ConfigManager::load()
            .concurrency
            .unwrap_or(1)
            .clamp(1, 3) as usize;

        loop {
            let task = {
                let mut st = self.state.lock().unwrap();
                if st.paused {
                    break;
                }
                if st.running.len() >= concurrency {
                    break;
                }
                match st.queued.pop_front() {
                    Some(t) => {
                        st.running.push(t.clone());
                        Some(t)
                    }
                    None => break,
                }
            };
            let Some(task) = task else { break };
            let id = task.id.clone();
            let _ = self
                .app
                .emit("download-started", serde_json::json!({ "task_id": id }));
            let queue = self.clone();
            // Use Tauri's runtime so spawning works even from synchronous
            // commands (e.g. start_queue / enqueue_download) that run outside
            // the tokio runtime context — tokio::spawn would panic there.
            tauri::async_runtime::spawn(async move { queue.run_worker(task).await });
        }
    }

    /// Start draining the queue (used by batch mode after the user presses
    /// "开始任务"). Clears the paused flag first.
    pub fn start(&self) {
        self.state.lock().unwrap().paused = false;
        self.pump();
    }

    /// Pause the queue: no new tasks start, currently running tasks finish.
    pub fn pause(&self) {
        self.state.lock().unwrap().paused = true;
    }

    /// Resume a paused queue.
    pub fn resume(&self) {
        self.state.lock().unwrap().paused = false;
        self.pump();
    }

    /// Whether the queue is paused (used by tests / future UI states).
    pub fn is_paused(&self) -> bool {
        self.state.lock().unwrap().paused
    }

    /// Execute one task with retry handling, then record history, emit
    /// `download-finished` and pump the next task. A task interrupted by a
    /// per-task pause is moved to the paused list (cache kept for resume).
    async fn run_worker(self, task: QueuedTask) {
        let id = task.id.clone();
        let preserve_cache = task.resume;
        let mut retries = ConfigManager::load().retry_count.unwrap_or(0);
        let mut attempts: u8 = 0;
        let mut last_error: Option<String> = None;
        let mut saved: Option<String> = None;

        loop {
            attempts = attempts.saturating_add(1);
            let config = task.config.clone();
            let app = self.app.clone();
            let tid = id.clone();
            let progress_cb = move |p: DownloadProgress| {
                let mut payload =
                    serde_json::to_value(&p).unwrap_or(serde_json::json!({}));
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("task_id".into(), serde_json::json!(tid));
                }
                let _ = app.emit("download-progress", payload);
            };

            match self
                .downloader
                .download(&id, preserve_cache, &config, progress_cb)
                .await
            {
                Ok(Some(path)) => {
                    saved = Some(path);
                    break;
                }
                Ok(None) => {
                    last_error = Some("下载失败（无详细信息）".to_string());
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }

            // A user-initiated cancel / pause must not be retried.
            if last_error
                .as_deref()
                .is_some_and(|e| e.contains("用户主动取消"))
            {
                break;
            }
            if retries > 0 {
                retries -= 1;
                continue;
            }
            break;
        }

        // Remove from the running list.
        {
            let mut st = self.state.lock().unwrap();
            st.running.retain(|t| t.id != id);
            // 该任务已被 pause_task 同步移入暂停列表（kill 后走到这里）：
            // 保留 cache 供续传，直接结束，不写历史、不 emit finished。
            if st.paused_tasks.iter().any(|t| t.id == id) {
                let _ = self
                    .app
                    .emit("download-paused", serde_json::json!({ "task_id": id }));
                drop(st);
                self.persist();
                self.pump();
                return;
            }
        }

        let cancelled = last_error
            .as_deref()
            .is_some_and(|e| e.contains("用户主动取消"));

        // 写历史时优先使用 fetch 到的卡片信息（task.info，信息获取阶段由前端
        // 回写持久化），缺失时回退到入队时的 config 快照。批量任务入队时
        // config 元数据为空，若不合并 task.info，下载完成的历史卡片会没有
        // 封面/作者/播放/点赞。
        let info = task.info.as_ref();
        let h_title = info
            .and_then(|v| v.get("title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(task.title.clone());
        let h_thumbnail = info
            .and_then(|v| v.get("thumbnail"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(task.config.thumbnail.clone());
        let h_uploader = info
            .and_then(|v| v.get("uploader"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(task.config.uploader.clone());
        let info_duration = info
            .and_then(|v| v.get("duration"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let info_views = info
            .and_then(|v| v.get("view_count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let info_likes = info
            .and_then(|v| v.get("like_count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let h_duration = if info_duration > 0 {
            info_duration
        } else {
            task.config.duration
        };
        let h_views = if info_views > 0 {
            info_views
        } else {
            task.config.view_count
        };
        let h_likes = if info_likes > 0 {
            info_likes
        } else {
            task.config.like_count
        };

        // Record history (only real outcomes; user-cancelled tasks are not
        // written to history).
        if !cancelled {
            if let Some(video_id) = task.config.video_id.as_deref() {
                if let Some(path) = saved.clone() {
                    let _ = DownloadHistory::record(
                        video_id,
                        h_title,
                        h_thumbnail,
                        Some(task.config.url.clone()),
                        h_uploader,
                        h_duration,
                        h_views,
                        h_likes,
                        Some(path.clone()),
                    );
                    // 主动获取文件大小并写入历史（显示在下载时间后）。
                    if let Ok(meta) = std::fs::metadata(&path) {
                        let _ = DownloadHistory::record_file_size(video_id, meta.len() as i64);
                    }
                } else {
                    let _ = DownloadHistory::record_failed(
                        video_id,
                        h_title,
                        h_thumbnail,
                        Some(task.config.url.clone()),
                        h_uploader,
                        h_duration,
                        h_views,
                        h_likes,
                        last_error.clone().unwrap_or_default(),
                        attempts,
                    );
                }
            }
        }

        let status = if saved.is_some() {
            "completed"
        } else if cancelled {
            "cancelled"
        } else {
            "failed"
        };
        let _ = self.app.emit(
            "download-finished",
            serde_json::json!({
                "task_id": id,
                "status": status,
                "error": last_error,
                "file_path": saved,
                "attempts": attempts,
            }),
        );

        // Persist the remaining queue, then keep draining.
        self.persist();
        self.pump();
    }

    /// Cancel a task: remove it from the queue if it is still waiting, or
    /// terminate its process if it is running.
    pub fn cancel_task(&self, task_id: &str) {
        {
            let mut st = self.state.lock().unwrap();
            st.queued.retain(|t| t.id != task_id);
            st.paused_tasks.retain(|t| t.id != task_id);
        }
        self.downloader.cancel_task(task_id);
        self.persist();
    }

    /// Remove all queued (not yet started) tasks. Running tasks finish.
    pub fn clear_queued(&self) {
        let mut st = self.state.lock().unwrap();
        st.queued.clear();
        st.paused_tasks.clear();
        self.persist();
    }

    /// Cancel ALL active tasks: drop every queued / paused task and terminate
    /// every running process. Finished downloads (history) are untouched.
    pub fn cancel_all(&self) {
        let running_ids: Vec<String> = {
            let mut st = self.state.lock().unwrap();
            st.queued.clear();
            st.paused_tasks.clear();
            st.running.iter().map(|t| t.id.clone()).collect()
        };
        for id in running_ids {
            self.downloader.cancel_task(&id);
        }
        self.persist();
    }

    /// Pause a single task (synchronous):
    /// - queued task → moved to the paused list (never started).
    /// - running task → moved to the paused list immediately and its process
    ///   killed; the cache directory (.part) is kept for resume. The worker
    ///   finishes and, finding the task already in the paused list, ends
    ///   quietly (no finished event / history).
    pub fn pause_task(&self, task_id: &str) {
        let mut moved = false;
        {
            let mut st = self.state.lock().unwrap();
            if let Some(pos) = st.queued.iter().position(|t| t.id == task_id) {
                let t = st.queued.remove(pos).unwrap();
                st.paused_tasks.push_back(t);
                moved = true;
            } else if let Some(pos) = st.running.iter().position(|t| t.id == task_id) {
                let t = st.running.remove(pos); // Vec::remove 直接返回元素
                st.paused_tasks.push_back(t);
                moved = true;
            }
        }
        if moved {
            self.downloader.cancel_task(task_id);
            self.persist();
        }
    }

    /// Resume a paused task: move it back to the queue with `resume = true`
    /// (cache is kept, so yt-dlp resumes from the .part file).
    pub fn resume_task(&self, task_id: &str) {
        {
            let mut st = self.state.lock().unwrap();
            if let Some(pos) = st.paused_tasks.iter().position(|t| t.id == task_id) {
                let mut t = st.paused_tasks.remove(pos).unwrap();
                t.resume = true;
                st.queued.push_back(t);
            }
        }
        self.persist();
        self.pump();
    }

    /// Pause every active task (queued → paused; running → killed and moved to
    /// paused synchronously). Each task emits `download-paused`.
    pub fn pause_all(&self) {
        let ids: Vec<String> = {
            let st = self.state.lock().unwrap();
            st.queued
                .iter()
                .chain(st.running.iter())
                .map(|t| t.id.clone())
                .collect()
        };
        for id in ids {
            self.pause_task(&id);
        }
    }

    /// Resume every paused task (back to the queue, resumed from cache).
    pub fn resume_all(&self) {
        let ids: Vec<String> = {
            let st = self.state.lock().unwrap();
            st.paused_tasks.iter().map(|t| t.id.clone()).collect()
        };
        {
            let mut st = self.state.lock().unwrap();
            st.paused = false; // 清全局暂停标志，确保 pump 能启动
        }
        for id in ids {
            self.resume_task(&id);
        }
    }

    /// Snapshot of queued + running + paused tasks for the frontend.
    pub fn status(&self) -> Vec<serde_json::Value> {
        let st = self.state.lock().unwrap();
        let mut items = Vec::new();
        for t in st.running.iter() {
            items.push(serde_json::json!({
                "task_id": t.id,
                "seq": t.seq,
                "url": t.config.url,
                "title": t.title,
                "status": "downloading",
                "info": t.info,
            }));
        }
        for t in st.paused_tasks.iter() {
            items.push(serde_json::json!({
                "task_id": t.id,
                "seq": t.seq,
                "url": t.config.url,
                "title": t.title,
                "status": "paused",
                "info": t.info,
            }));
        }
        for t in st.queued.iter() {
            items.push(serde_json::json!({
                "task_id": t.id,
                "seq": t.seq,
                "url": t.config.url,
                "title": t.title,
                "status": "queued",
                "info": t.info,
            }));
        }
        items
    }

    // ==================== Persistence ====================

    /// config/queue.json — persisted pending + running tasks (only when the
    /// "persist queue" setting is enabled).
    fn queue_file() -> std::path::PathBuf {
        crate::utils::app_home::AppHome::config_dir().join("queue.json")
    }

    fn persist_enabled() -> bool {
        ConfigManager::load().queue_persist.unwrap_or(false)
    }

    /// Write the current queued + running tasks to disk (no-op unless the
    /// setting is enabled). Called on every queue mutation.
    fn persist(&self) {
        if !Self::persist_enabled() {
            return;
        }
        self.persist_now();
    }

    /// Force-write the current queue to disk regardless of the persist setting
    /// (used by "保存进度并退出").
    pub fn save_now(&self) {
        self.persist_now();
    }

    /// Update a task's card metadata (thumbnail / uploader / duration / …).
    /// Searches queued / running / paused lists so the new info is persisted
    /// (via the persist setting or "保存进度并退出") and survives a restart.
    /// No-op when the task id is not found.
    pub fn update_info(&self, id: &str, info: Option<serde_json::Value>) {
        {
            let mut st = self.state.lock().unwrap();
            let mut pending = Some(info);
            if let Some(t) = st.queued.iter_mut().find(|t| t.id == id) {
                t.info = pending.take().flatten();
            } else if let Some(t) = st.running.iter_mut().find(|t| t.id == id) {
                t.info = pending.take().flatten();
            } else if let Some(t) = st.paused_tasks.iter_mut().find(|t| t.id == id) {
                t.info = pending.take().flatten();
            }
        }
        self.persist();
    }

    /// Whether there are any active tasks (queued / running / paused).
    /// Used by the exit-confirmation flow.
    pub fn has_active(&self) -> bool {
        let st = self.state.lock().unwrap();
        !st.queued.is_empty() || !st.running.is_empty() || !st.paused_tasks.is_empty()
    }

    fn persist_now(&self) {
        let st = self.state.lock().unwrap();
        let mut tasks: Vec<QueuedTask> = Vec::new();
        // 记录每个任务在持久化时的状态，重启后据此还原。
        for t in st.queued.iter() {
            let mut x = t.clone();
            x.status = "queued".to_string();
            tasks.push(x);
        }
        for t in st.running.iter() {
            let mut x = t.clone();
            x.status = "running".to_string();
            tasks.push(x);
        }
        for t in st.paused_tasks.iter() {
            let mut x = t.clone();
            x.status = "paused".to_string();
            tasks.push(x);
        }
        drop(st);
        if let Ok(json) = serde_json::to_string_pretty(&tasks) {
            let _ = std::fs::write(Self::queue_file(), json);
        }
    }

    /// Restore persisted tasks at startup. A `config/queue.json` left by the
    /// "保存进度并退出" flow (or the persist setting) is re-enqueued and the
    /// queue starts draining.
    pub fn restore_if_enabled(&self) {
        let file = Self::queue_file();
        if !file.exists() {
            return;
        }
        let tasks: Vec<QueuedTask> = std::fs::read_to_string(&file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let _ = std::fs::remove_file(&file);

        if tasks.is_empty() {
            return;
        }
        {
            let mut st = self.state.lock().unwrap();
            for t in tasks {
                note_restored_id(&t.id);
                let dup = st
                    .queued
                    .iter()
                    .chain(st.running.iter())
                    .chain(st.paused_tasks.iter())
                    .any(|x| x.config.url == t.config.url);
                if dup {
                    continue;
                }
                let _ = self.app.emit(
                    "download-queued",
                    serde_json::json!({
                        "task_id": t.id,
                        "url": t.config.url,
                        "title": t.title,
                        "status": if t.status == "paused" { "paused" } else { "queued" },
                        "info": t.info,
                    }),
                );
                // 按持久化时的状态还原：
                // - paused → 保持暂停（不启动）
                // - running → 回排队并 resume=true（续传 .part，继续下载）
                // - queued / 旧数据 → 正常排队
                match t.status.as_str() {
                    "paused" => {
                        st.paused_tasks.push_back(t);
                    }
                    "running" => {
                        let mut x = t;
                        x.resume = true;
                        st.queued.push_back(x);
                    }
                    _ => {
                        st.queued.push_back(t);
                    }
                }
            }
        }
        self.pump();
    }
}
