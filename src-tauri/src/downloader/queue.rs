//! Multi-task download queue.
//!
//! Independent from the single-download path (download page): tasks enqueued
//! here run concurrently up to the configured concurrency. Staging directories
//! live in `download_cache/{cache_key}/` (keyed by URL + format — see
//! `YtDlpDownloader::cache_key`), so retried / re-enqueued / resumed tasks
//! resume from the previous `.part` file. Retries, dedup and cancellation are
//! handled here.

use crate::downloader::ytdlp::YtDlpDownloader;
use crate::models::config::DownloadConfig;
use crate::models::progress::DownloadProgress;
use crate::services::config::ConfigManager;
use crate::services::download_history::DownloadHistory;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

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
    /// True when this task is being resumed after a pause / restart. Used only
    /// for the pause→resume worker race guard (see `run_worker`); the downloader
    /// itself always resumes from the `.part` file via the stable cache key.
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
    /// Task source: 0=单链(single) 1=批量(batch) 2=书签(bookmark).
    /// `0` doubles as the default for legacy / unknown tasks.
    #[serde(default)]
    pub source: i64,
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
    /// Task id → index it occupied in `paused_tasks` before being resumed.
    /// Lets a task paused again land back near its original slot instead of
    /// jumping to the end of the paused list. Memory only (not persisted).
    paused_slots: HashMap<String, usize>,
}

/// Add a task to the paused list. When a remembered slot exists (the task was
/// resumed before), insert it back near that slot; otherwise append to the end.
fn insert_paused(st: &mut QueueState, task: QueuedTask, slot: Option<usize>) {
    let len = st.paused_tasks.len();
    match slot {
        Some(s) => st.paused_tasks.insert(s.min(len), task),
        None => st.paused_tasks.push_back(task),
    }
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
        source: i64,
    ) -> Result<String, String> {
        // 去重与存储统一使用 trim 后的 URL，避免带尾随空格的链接绕过判重。
        let mut config = config;
        let url = config.url.trim().to_string();
        if url.is_empty() {
            return Err("链接不能为空".to_string());
        }
        config.url = url.clone();
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
                source,
            });
        }
        let _ = self.app.emit(
            "download-queued",
            serde_json::json!({
                "task_id": id, "url": url, "title": title, "info": info,
                "source": crate::services::download_history::source_name(source),
            }),
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
    async fn run_worker(self, mut task: QueuedTask) {
        let id = task.id.clone();
        let mut retries = ConfigManager::load().retry_count.unwrap_or(0);
        let mut attempts: u8 = 0;
        let mut last_error: Option<String> = None;
        // 所有成功移动的文件路径（多 media 推文有多个）；主路径 = 第一个。
        let mut saved_paths: Vec<String> = Vec::new();

        loop {
            attempts = attempts.saturating_add(1);
            // 前端两阶段信息获取（fetchVideoInfo）会把 info 回写到
            // st.running 里的任务对象；worker 持有的是 pump 入队时的旧快照
            // （两阶段路径 info=None）。这里每次尝试前刷新，确保 record()
            // 能读到完整信息（含 duration），而非依赖 meta 兜底。
            if let Some(latest) = self
                .state
                .lock()
                .unwrap()
                .running
                .iter()
                .find(|t| t.id == id)
            {
                task = latest.clone();
            }
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
                .download(&id, &config, progress_cb)
                .await
            {
                Ok(paths) if !paths.is_empty() => {
                    saved_paths = paths;
                    break;
                }
                Ok(_) => {
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
            // 竞态防护：若该任务已被 resume 并由新 worker 接管（running 中
            // 存在 resume=true 的同 id 任务），旧 worker 不删除新实例、不写
            // 历史、不 emit finished，静默结束（避免 UI 卡片被移除后又因新
            // worker 加回的闪烁，也避免重复事件）。
            if st.running.iter().any(|t| t.id == id && t.resume) {
                drop(st);
                self.persist();
                self.pump();
                return;
            }
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

        // 兜底：前端信息与入队 config 都缺失时（深链/扩展入队、fetch 失败），
        // 用 yt-dlp 下载时输出的真实元数据填充历史记录。
        let meta = self.downloader.read_task_meta(&task.id);
        let h_title = h_title.or_else(|| meta.as_ref().and_then(|m| m.title.clone()));
        let h_thumbnail =
            h_thumbnail.or_else(|| meta.as_ref().and_then(|m| m.thumbnail.clone()));
        let h_uploader = h_uploader.or_else(|| meta.as_ref().and_then(|m| m.uploader.clone()));
        let h_duration = if h_duration > 0 {
            h_duration
        } else {
            meta.as_ref().and_then(|m| m.duration).unwrap_or(0)
        };
        let h_views = if h_views > 0 {
            h_views
        } else {
            meta.as_ref().and_then(|m| m.view_count).unwrap_or(0)
        };
        let h_likes = if h_likes > 0 {
            h_likes
        } else {
            meta.as_ref().and_then(|m| m.like_count).unwrap_or(0)
        };

        // Record history (only real outcomes; user-cancelled tasks are not
        // written to history).
        if !cancelled {
            if let Some(video_id) = task.config.video_id.as_deref() {
                // 主路径 = 第一个文件（兼容历史页「打开文件位置」/ is_downloaded）。
                let main_path = saved_paths.first().cloned();
                if let Some(path) = main_path {
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
                        saved_paths.clone(),
                        task.source,
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
                        task.source,
                    );
                }
            }
        }

        let status = if !saved_paths.is_empty() {
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
                "file_path": saved_paths.first(),
                "attempts": attempts,
            }),
        );

        // Persist the remaining queue, then keep draining.
        self.persist();
        self.pump();
    }

    /// Cancel a task: remove it from the queue if it is still waiting, or
    /// terminate its process if it is running. The task's staged cache
    /// (`.part` / `.ytdl` / fragment files) is deleted too — this is an
    /// explicit deletion, so the resume cache is no longer needed.
    pub fn cancel_task(&self, task_id: &str) {
        let config: Option<DownloadConfig> = {
            let mut st = self.state.lock().unwrap();
            // running 任务不在 queued/paused 里，需要一起找才能拿到 config。
            let cfg = st
                .queued
                .iter()
                .chain(st.paused_tasks.iter())
                .chain(st.running.iter())
                .find(|t| t.id == task_id)
                .map(|t| t.config.clone());
            st.queued.retain(|t| t.id != task_id);
            st.paused_tasks.retain(|t| t.id != task_id);
            // running 也要立即移除，否则重新添加同 URL 会被「链接已在队列中」
            // 拒绝（worker 异步退场前任务一直残留在 running 列表）。
            st.running.retain(|t| t.id != task_id);
            cfg
        };
        self.downloader.cancel_task(task_id);
        if let Some(cfg) = config {
            self.downloader.cleanup_task_cache(&cfg);
        }
        self.persist();
    }

    /// Remove all queued (not yet started) tasks. Running tasks finish.
    pub fn clear_queued(&self) {
        let mut st = self.state.lock().unwrap();
        st.queued.clear();
        st.paused_tasks.clear();
        st.paused_slots.clear();
        self.persist();
    }

    /// Move a queued OR paused task to a new position within its own list.
    ///
    /// - Running tasks cannot be reordered (they are downloading).
    /// - Reordering a queued task changes the download order.
    /// - Reordering a paused task changes the order they are restored in when
    ///   resumed (paused tasks join the queue back in their list order).
    /// - `new_index` is clamped to `[0, len)`; `0` = top.
    /// - `seq` of every task in the affected list is rewritten to its new
    ///   position so the frontend's ordering matches.
    pub fn reorder_queue(&self, task_id: &str, new_index: usize) -> bool {
        let mut st = self.state.lock().unwrap();

        // Which list holds the task? Queued first (the common case).
        let target_is_queued = st.queued.iter().any(|t| t.id == task_id);
        if !target_is_queued && !st.paused_tasks.iter().any(|t| t.id == task_id) {
            return false;
        }

        if target_is_queued {
            let pos = st.queued.iter().position(|t| t.id == task_id).unwrap();
            let task = st.queued.remove(pos).unwrap();
            let len = st.queued.len();
            let target = new_index.min(len);
            st.queued.insert(target, task);
            for (i, t) in st.queued.iter_mut().enumerate() {
                t.seq = i as u64;
            }
        } else {
            let pos = st
                .paused_tasks
                .iter()
                .position(|t| t.id == task_id)
                .unwrap();
            let task = st.paused_tasks.remove(pos).unwrap();
            let len = st.paused_tasks.len();
            let target = new_index.min(len);
            st.paused_tasks.insert(target, task);
            for (i, t) in st.paused_tasks.iter_mut().enumerate() {
                t.seq = i as u64;
            }
            // 手动重排后位置已变，旧 slot 记录失效。
            st.paused_slots.clear();
        }

        drop(st);
        self.persist();
        true
    }

    /// Cancel ALL active tasks: drop every queued / paused task and terminate
    /// every running process. Finished downloads (history) are untouched.
    pub fn cancel_all(&self) {
        let (running_ids, configs): (Vec<String>, Vec<DownloadConfig>) = {
            let mut st = self.state.lock().unwrap();
            let configs = st
                .queued
                .iter()
                .chain(st.paused_tasks.iter())
                .chain(st.running.iter())
                .map(|t| t.config.clone())
                .collect();
            let running_ids = st.running.iter().map(|t| t.id.clone()).collect();
            st.queued.clear();
            st.paused_tasks.clear();
            st.paused_slots.clear();
            (running_ids, configs)
        };
        for id in running_ids {
            self.downloader.cancel_task(&id);
        }
        // 全部删除：逐个清理任务缓存。
        for cfg in configs {
            self.downloader.cleanup_task_cache(&cfg);
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
            // 该任务之前暂停过（被单启过）：恢复时记录了它离开暂停区的位置，
            // 重新暂停时插回原位附近，而不是追加到末尾。
            let slot = st.paused_slots.remove(task_id);
            if let Some(pos) = st.queued.iter().position(|t| t.id == task_id) {
                let t = st.queued.remove(pos).unwrap();
                insert_paused(&mut st, t, slot);
                moved = true;
            } else if let Some(pos) = st.running.iter().position(|t| t.id == task_id) {
                let t = st.running.remove(pos); // Vec::remove 直接返回元素
                insert_paused(&mut st, t, slot);
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
                // 记住离开位置，再次暂停时可插回原位而不是跑到末尾。
                st.paused_slots.insert(task_id.to_string(), pos);
                t.resume = true;
                // 恢复单个任务即解除全局暂停模式，否则 pump 不会启动它。
                st.paused = false;
                st.queued.push_back(t);
            }
        }
        self.persist();
        self.pump();
    }

    /// Pause every active task **atomically**, preserving the exact order the
    /// user saw before pausing.
    ///
    /// The whole `running + queued → paused` move happens in one lock hold, so
    /// concurrent worker completion cannot interleave and scramble the order.
    /// Running tasks are then killed outside the lock. `resume_all` restores
    /// `paused_tasks` in this same order, so "pause all → resume all" leaves
    /// the queue (and the download order) unchanged.
    pub fn pause_all(&self) {
        let running_ids: Vec<String> = {
            let mut st = self.state.lock().unwrap();
            st.paused = true;
            // 全部暂停重建暂停区，旧 slot 记录失效。
            st.paused_slots.clear();
            // running 在前、queued 在后 —— 与 status() 返回的展示顺序一致。
            let running: Vec<QueuedTask> = st.running.drain(..).collect();
            let queued: Vec<QueuedTask> = st.queued.drain(..).collect();
            let ids: Vec<String> = running.iter().chain(queued.iter()).map(|t| t.id.clone()).collect();
            st.paused_tasks.extend(running);
            st.paused_tasks.extend(queued);
            ids
        };
        // 锁外 kill 所有原 running 进程（cancel_task 内部处理 pid 清理）。
        for id in &running_ids {
            self.downloader.cancel_task(id);
        }
        self.persist();
    }

    /// Resume every paused task, restoring exactly the pre-pause order.
    ///
    /// `pause_all` moves queued tasks into `paused_tasks` first, then running
    /// tasks — so `paused_tasks` already holds the original order. Restoring
    /// them in that same order (back to the queue) keeps the queue identical
    /// to what the user saw before pausing; the frontend renders the queue in
    /// backend order, so display and download order both stay stable.
    pub fn resume_all(&self) {
        // 先收集 id 释放借用，再逐一移出（避免 drain 与 push_back 双重可变借用）。
        let ids: Vec<String> = {
            let st = self.state.lock().unwrap();
            st.paused_tasks.iter().map(|t| t.id.clone()).collect()
        };
        {
            let mut st = self.state.lock().unwrap();
            st.paused = false; // 清全局暂停标志，确保 pump 能启动
            // 全部恢复后暂停区清空，slot 记录不再有意义。
            st.paused_slots.clear();
            for id in ids {
                if let Some(pos) = st.paused_tasks.iter().position(|t| t.id == id) {
                    let mut t = st.paused_tasks.remove(pos).unwrap();
                    t.resume = true;
                    st.queued.push_back(t);
                }
            }
        }
        self.persist();
        self.pump();
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
                "source": crate::services::download_history::source_name(t.source),
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
                "source": crate::services::download_history::source_name(t.source),
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
                "source": crate::services::download_history::source_name(t.source),
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
        match serde_json::to_string_pretty(&tasks) {
            Ok(json) => match std::fs::write(Self::queue_file(), &json) {
                Ok(_) => debug!(
                    "persisted queue to {} ({} tasks, {} bytes)",
                    Self::queue_file().display(),
                    tasks.len(),
                    json.len()
                ),
                Err(e) => warn!(
                    "failed to persist queue to {}: {}",
                    Self::queue_file().display(),
                    e
                ),
            },
            Err(e) => warn!("failed to serialize queue for persistence: {}", e),
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
        let raw = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => {
                warn!("failed to read queue file {}: {}", file.display(), e);
                let _ = std::fs::remove_file(&file);
                return;
            }
        };
        let tasks: Vec<QueuedTask> = match serde_json::from_str(&raw) {
            Ok(t) => t,
            Err(e) => {
                warn!(
                    "failed to parse queue file {} ({} bytes): {}",
                    file.display(),
                    raw.len(),
                    e
                );
                let _ = std::fs::remove_file(&file);
                return;
            }
        };
        let _ = std::fs::remove_file(&file);

        if tasks.is_empty() {
            debug!("queue file {} contained no tasks, nothing to restore", file.display());
            return;
        }
        let mut restored = 0usize;
        let mut skipped = 0usize;
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
                    skipped += 1;
                    continue;
                }
                restored += 1;
                let _ = self.app.emit(
                    "download-queued",
                    serde_json::json!({
                        "task_id": t.id,
                        "url": t.config.url,
                        "title": t.title,
                        "status": if t.status == "paused" { "paused" } else { "queued" },
                        "info": t.info,
                        "source": crate::services::download_history::source_name(t.source),
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
        info!(
            "restored {} persisted task(s) from {} (skipped {} duplicate(s))",
            restored,
            file.display(),
            skipped
        );
        self.pump();
    }
}
