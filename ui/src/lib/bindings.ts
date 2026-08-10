import { invoke } from "@tauri-apps/api/core";
import type {
  VideoInfo,
  DownloadConfig,
  DownloadProgress,
  AppSettings,
  ProxyStatus,
  ProxyTestResult,
  CookiesValidationResult,
  ToolStatus,
  DownloadHistoryItem,
} from "./types";

// --- Misc helpers ---

/** Build a proxy URL string from settings (`scheme://host:port`), or
 *  `undefined` when the proxy is not configured. Used by the app updater
 *  `check({ proxy })` (official tauri-plugin-updater). */
export function buildProxyUrl(s: AppSettings): string | undefined {
  const host = s.proxy_host?.trim();
  if (!host) return undefined;
  const port = s.proxy_port || 0;
  const scheme = s.proxy_scheme?.trim() || "http";
  return `${scheme}://${host}:${port}`;
}

// --- Download ---

export async function fetchVideoInfo(url: string): Promise<VideoInfo> {
  return invoke("fetch_video_info", { url });
}

export interface DownloadStatus {
  downloaded: boolean;
  downloaded_at: number | null;
  file_path: string | null;
}

export async function checkVideoDownloaded(videoId: string): Promise<DownloadStatus> {
  return invoke("check_video_downloaded", { videoId });
}

// --- Download queue ---

/** 入队一个批量下载任务，返回 task_id（URL 已去重）。
 *  autoStart=false 时任务仅等待 startQueue 才开始。 */
export async function enqueueDownload(
  config: DownloadConfig,
  title?: string | null,
  autoStart?: boolean,
  info?: unknown
): Promise<string> {
  return invoke("enqueue_download", {
    config,
    title,
    autoStart: autoStart ?? true,
    info,
  });
}

/** 开始运行多任务队列（批量模式「开始任务」）。 */
export async function startQueue(): Promise<void> {
  return invoke("start_queue");
}

/** 暂停多任务队列：不再启动新任务，运行中的任务继续完成。 */
export async function pauseQueue(): Promise<void> {
  return invoke("pause_queue");
}

/** 恢复暂停的多任务队列。 */
export async function resumeQueue(): Promise<void> {
  return invoke("resume_queue");
}

/** 暂停单个任务（排队任务移出；下载中任务终止并保留缓存续传）。 */
export async function pauseQueueTask(taskId: string): Promise<void> {
  return invoke("pause_queue_task", { taskId });
}

/** 继续一个已暂停的任务（从保留的缓存续传）。 */
export async function resumeQueueTask(taskId: string): Promise<void> {
  return invoke("resume_queue_task", { taskId });
}

/** 暂停全部活跃任务（每个任务 emit download-paused）。 */
export async function pauseAllTasks(): Promise<void> {
  return invoke("pause_all_tasks");
}

/** 恢复全部已暂停任务。 */
export async function resumeAllTasks(): Promise<void> {
  return invoke("resume_all_tasks");
}

/** 取消一个排队中 / 运行中的多任务下载。 */
export async function cancelQueueTask(taskId: string): Promise<void> {
  return invoke("cancel_queue_task", { taskId });
}

/** 清空仍在排队的多任务（运行中的任务会继续完成）。 */
export async function clearDownloadQueue(): Promise<void> {
  return invoke("clear_download_queue");
}

/** 取消全部活跃任务（排队/暂停/运行中）；下载完成的历史记录不受影响。 */
export async function cancelAllTasks(): Promise<void> {
  return invoke("cancel_all_tasks");
}

/** 获取多任务队列快照（排队 + 运行中）。 */
export async function queueStatus(): Promise<QueueItem[]> {
  return invoke("queue_status");
}

/** 更新任务的卡片元数据（封面/作者/时长等），持久化到后端，重启后保留。 */
export async function updateTaskInfo(
  taskId: string,
  info?: unknown
): Promise<void> {
  return invoke("update_task_info", { taskId, info: info ?? null });
}

export interface QueueItem {
  task_id: string;
  /** 入队序号（稳定顺序，前端据此排序展示）。 */
  seq?: number;
  url: string;
  title: string | null;
  status: "queued" | "downloading" | "paused";
  /** 后端持久化的卡片信息（保存进度重启后恢复）。 */
  info?: unknown;
}

export async function listDownloadHistory(): Promise<DownloadHistoryItem[]> {
  return invoke("list_download_history");
}

export async function deleteDownloadHistory(id: string): Promise<void> {
  return invoke("delete_download_history", { id });
}

export async function clearDownloadHistory(): Promise<void> {
  return invoke("clear_download_history");
}

// --- Settings ---

export async function loadSettings(): Promise<AppSettings> {
  return invoke("load_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function saveSettingsToPath(settings: AppSettings, path: string): Promise<void> {
  return invoke("save_settings_to_path", { settings, path });
}

export async function loadSettingsFromPath(path: string): Promise<AppSettings> {
  return invoke("load_settings_from_path", { path });
}

export async function applyAndPersistSettings(settings: AppSettings): Promise<void> {
  return invoke("apply_and_persist_settings", { settings });
}

export async function applyDefaultConfig(): Promise<AppSettings> {
  return invoke("apply_default_config");
}

export async function saveAsDefault(settings: AppSettings): Promise<void> {
  return invoke("save_as_default", { settings });
}

export async function getDownloadDir(): Promise<string> {
  return invoke("get_download_dir");
}

export async function getConfigPath(): Promise<string> {
  return invoke("get_config_path");
}

export async function applySavedProxy(): Promise<boolean> {
  return invoke("apply_saved_proxy");
}

export async function loadSavedCookies(): Promise<[string | null, string | null]> {
  return invoke("load_saved_cookies");
}

export async function saveAndApplyCookies(browser: string | null): Promise<void> {
  return invoke("save_and_apply_cookies", { browser });
}

export async function applySavedCookies(): Promise<void> {
  return invoke("apply_saved_cookies");
}

export async function saveLanguage(lang: string): Promise<void> {
  return invoke("save_language", { lang });
}

// --- Disclaimer ---

export async function getDisclaimerAccepted(): Promise<boolean> {
  return invoke("get_disclaimer_accepted");
}

export async function acceptDisclaimer(): Promise<void> {
  return invoke("accept_disclaimer");
}

// --- Proxy ---

export async function testProxy(
  host: string,
  port: number,
  scheme?: string
): Promise<ProxyTestResult> {
  return invoke("test_proxy", { host, port, scheme });
}

export async function getProxyStatus(): Promise<ProxyStatus> {
  return invoke("get_proxy_status");
}

export async function setProxyMode(enabled: boolean): Promise<void> {
  return invoke("set_proxy_mode", { enabled });
}

// --- Cookies ---

export async function validateCookies(browser: string): Promise<CookiesValidationResult> {
  return invoke("validate_cookies", { browser });
}

export async function scanCookies(): Promise<string | null> {
  return invoke("scan_cookies");
}

// --- Bootstrap ---

export async function checkYtdlp(): Promise<ToolStatus> {
  return invoke("check_ytdlp");
}

export async function checkFfmpeg(): Promise<ToolStatus> {
  return invoke("check_ffmpeg");
}

/** Whether the bundled ffmpeg (bin/ffmpeg.exe) exists on disk. */
export async function isFfmpegBundled(): Promise<boolean> {
  return invoke("is_ffmpeg_bundled");
}

export async function downloadYtDlp(): Promise<string> {
  return invoke("download_ytdlp");
}

export async function downloadFfmpeg(): Promise<string> {
  return invoke("download_ffmpeg");
}

export async function pingGoogle(): Promise<boolean> {
  return invoke("ping_google");
}

export async function cancelBootstrapDownload(): Promise<void> {
  return invoke("cancel_bootstrap_download");
}

export async function getBinDir(): Promise<string> {
  return invoke("get_bin_dir");
}

export async function getRootDir(): Promise<string> {
  return invoke("get_root_dir");
}

export async function openRootDir(): Promise<void> {
  return invoke("open_root_dir");
}

export async function getConfigDir(): Promise<string> {
  return invoke("get_config_dir");
}

export async function openConfigDir(): Promise<void> {
  return invoke("open_config_dir");
}

export async function openLogsDir(): Promise<void> {
  return invoke("open_logs_dir");
}

export async function openDownloadDir(): Promise<void> {
  return invoke("open_download_dir");
}

export async function openDownloadPath(videoId: string): Promise<void> {
  return invoke("open_download_path", { videoId });
}

/** 在文件管理器中定位某个文件（下载完成 toast 点击标题用）。 */
export async function openFilePath(filePath: string): Promise<void> {
  return invoke("open_file_path", { filePath });
}

/** 获取应用版本号（数据源：Cargo.toml）。 */
export async function getVersion(): Promise<string> {
  return invoke("get_version");
}

/** 读取持久化的隐私模式状态。 */
export async function getPrivacyMode(): Promise<boolean> {
  return invoke("get_privacy_mode");
}

/** 持久化隐私模式状态。 */
export async function setPrivacyModePersist(enabled: boolean): Promise<void> {
  return invoke("set_privacy_mode", { enabled });
}

/** 退出应用。saveProgress=true 时先强制保存队列进度到 queue.json。 */
export async function quitApp(saveProgress?: boolean): Promise<void> {
  return invoke("quit_app", { saveProgress: saveProgress ?? false });
}

/** 是否有活跃任务（排队/下载中/暂停）——用于退出确认。 */
export async function hasActiveTasks(): Promise<boolean> {
  return invoke("has_active_tasks");
}

// --- Uninstall ---

export interface UninstallInfo {
  installed: boolean;
  uninstall_string: string | null;
  display_name: string | null;
}

export async function getUninstallInfo(): Promise<UninstallInfo> {
  return invoke("get_uninstall_info");
}

export async function uninstallApp(): Promise<boolean> {
  return invoke("uninstall_app");
}

export async function openUninstallPanel(): Promise<void> {
  return invoke("open_uninstall_panel");
}

export interface YtdlpUpdateResult {
  has_update: boolean;
  not_installed?: boolean;
  local_version: string | null;
  latest_version: string | null;
  url: string | null;
  error?: string;
}

export async function checkYtdlpUpdate(
  localVersion?: string
): Promise<YtdlpUpdateResult> {
  return invoke("check_ytdlp_update", { localVersion });
}

export interface FfmpegUpdateResult {
  has_update: boolean;
  not_installed?: boolean;
  local_version: string | null;
  latest_version: string | null;
  url: string | null;
  error?: string;
}

export async function checkFfmpegUpdate(): Promise<FfmpegUpdateResult> {
  return invoke("check_ffmpeg_update");
}
