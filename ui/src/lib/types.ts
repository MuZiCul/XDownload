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
}

// --- Download Config ---

export interface DownloadConfig {
  url: string;
  format_id: string;
  output_dir: string;
  output_template: string;
  extract_audio: boolean;
  embed_subtitles: boolean;
  embed_thumbnail: boolean;
  write_thumbnail: boolean;
  proxy: string | null;
  retries: number;
  socket_timeout: number;
  cookies_file: string | null;
  cookies_from_browser: string | null;
  max_height: number;
  download_archive: string | null;
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
  cookies_from_browser?: string;
  cookies_file?: string;
  lang?: string;
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
}

// --- Tools ---

export interface ToolStatus {
  available: boolean;
  version: string | null;
}

export interface BootstrapProgress {
  tool: string;
  percent: number;
}

export interface BootstrapComplete {
  tool: string;
  success: boolean;
}
