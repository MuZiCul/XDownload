// --- Video Info ---

export interface Format {
  format_id: string;
  ext: string | null;
  resolution: string | null;
  width: number | null;
  height: number | null;
  filesize: number | null;
  filesize_approx: number | null;
  tbr: number | null;
  fps: number | null;
  vcodec: string | null;
  acodec: string | null;
  format_note: string | null;
}

export interface VideoInfo {
  id: string;
  url: string;
  title: string | null;
  description: string | null;
  duration: number;
  thumbnail: string | null;
  uploader: string | null;
  view_count: number;
  like_count: number;
  webpage_url: string | null;
  formats: Format[];
  /** Number of media entries in this URL (1 for a normal video; >1 for multi-media tweets). */
  media_count?: number;
  // Download status (filled by the backend command layer)
  downloaded: boolean;
  downloaded_at: number | null;
  download_path: string | null;
}

// --- Download Status (backend check_video_downloaded) ---

export interface DownloadStatus {
  downloaded: boolean;
  downloaded_at: number | null;
  file_path: string | null;
}

// --- Download Config ---

export interface DownloadConfig {
  url: string;
  video_id: string | null;
  /** Video title, stored in download history for display. */
  title?: string | null;
  /** Video thumbnail URL, stored in download history for the cover. */
  thumbnail?: string | null;
  /** Video metadata, stored in download history for the history page. */
  uploader?: string | null;
  duration?: number;
  view_count?: number;
  like_count?: number;
  format_id: string;
  output_dir: string;
  output_template: string;
  extract_audio: boolean;
  embed_subtitles: boolean;
  embed_thumbnail: boolean;
  write_thumbnail: boolean;
  proxy: string | null;
  socket_timeout: number;
  cookies_from_browser: string | null;
  max_height: number;
  download_archive: string | null;
  /** yt-dlp --limit-rate (e.g. "1M"). Empty/null = unlimited. */
  download_rate_limit?: string | null;
}

// --- Progress ---

export interface DownloadProgress {
  downloaded_bytes: number;
  total_bytes: number;
  speed: string;
  eta: string;
  percent: string;
  status: string;
}

// --- Settings ---

export interface AppSettings {
  download_dir?: string;
  proxy_host?: string;
  proxy_port?: number;
  proxy_scheme?: string;
  cookies_from_browser?: string;
  lang?: string;
  /** 并发下载数（1-3，默认 1）。 */
  concurrency?: number;
  /** 失败重试次数（0-5，默认 0 = 不重试）。 */
  retry_count?: number;
  /** 队列持久化开关（默认关）。 */
  queue_persist?: boolean;
  /** 下载限速（yt-dlp --limit-rate），如 "1M"、"25M"。空 = 不限速。 */
  download_rate_limit?: string;
  /** HLS/DASH 分片并发下载数（yt-dlp --concurrent-fragments）。 */
  hls_concurrent_fragments?: number;
  /** HLS/DASH 分片失败重试次数（yt-dlp --fragment-retries）。 */
  hls_fragment_retries?: number;
  /** 断点续传开关（默认关）：开启时任务面板隐藏暂停/开始按钮。 */
  resume_support?: boolean;
}

// --- Bookmarks sync (manual) ---

/** 一条已解析的书签（同步预览弹窗展示）。 */
export interface BookmarksVideoItem {
  tweet_id: string;
  handle: string;
  url: string;
  text: string;
  author_name: string;
  has_video: boolean;
  /** 是否已下载（记录存在且文件在磁盘上）。已下载的也可勾选重新下载。 */
  downloaded: boolean;
}

/** 手动同步的预览结果（阶段一）：只展示，不触碰游标/队列。 */
export interface BookmarksSyncPreview {
  /** 当前 X 上书签总数。 */
  total: number;
  /** 新增书签总数（含纯文字/图片）。 */
  new_count: number;
  /** 所有含视频的书签（含已下载），等待用户勾选确认入队。 */
  video_items: BookmarksVideoItem[];
}

/** 确认入队的结果（阶段二）。 */
export interface BookmarksConfirmResult {
  queued_count: number;
}

/** 书签目录中的一条记录（来自 bookmarks 表，含视频与无视频）。 */
export interface BookmarksListItem {
  id: number;
  video_id: string;
  url: string;
  handle: string;
  author_name: string;
  title: string;
  has_video: boolean;
  downloaded: boolean;
  added_at: number;
}

// --- Network ---

/** GitHub 可达性检测结果（更新下载前预检）。 */
export interface GitHubReachability {
  /** 直连 github.com 是否成功。 */
  direct_ok: boolean;
  /** 是否配置了代理。 */
  proxy_configured: boolean;
  /** 走代理是否成功。 */
  proxy_ok: boolean;
  /** 最终是否可达（直连或代理任一成功）。 */
  reachable: boolean;
}

// --- Proxy ---

export interface ProxyStatus {
  enabled: boolean;
  host: string | null;
  port: number;
  from_system: boolean;
  proxy_string: string;
}

export interface ProxyTestResult {
  success: boolean;
  http_status: number;
  elapsed_ms: number;
  message: string;
}

// --- Cookies ---

export interface CookiesValidationResult {
  success: boolean;
  message: string;
  cookie_count: number;
  username?: string;
  /** Machine-readable failure reason for i18n (e.g. "token_invalid"). */
  error_code?: string | null;
}

// --- Tools ---

export interface ToolStatus {
  available: boolean;
  version: string | null;
}

/** A single download-history record (from the config/data.db downloads table). */
export interface DownloadHistoryItem {
  video_id: string;
  title: string | null;
  /** Video thumbnail URL (may be absent for legacy records). */
  thumbnail: string | null;
  /** Original video URL (may be absent for legacy records). */
  url: string | null;
  /** Video metadata (may be zero / null for legacy records). */
  uploader: string | null;
  duration: number;
  view_count: number;
  like_count: number;
  file_path: string | null;
  /** File size in bytes (filled after a successful download). */
  file_size: number | null;
  downloaded_at: number;
  /** Whether the saved file still exists on disk. */
  file_exists: boolean;
  /** Success | Failed (defaults to Success for legacy records). */
  status: "success" | "failed";
  /** Failure reason (when status = failed). */
  error: string | null;
  /** Number of download attempts including retries. */
  attempts: number;
}

export interface BootstrapProgress {
  tool: string;
  percent: number;
}

export interface BootstrapComplete {
  tool: string;
  success: boolean;
}
