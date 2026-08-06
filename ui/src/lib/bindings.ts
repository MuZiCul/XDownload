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
} from "./types";

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

export async function startDownload(config: DownloadConfig): Promise<boolean> {
  return invoke("start_download", { config });
}

export async function cancelDownload(): Promise<void> {
  return invoke("cancel_download");
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

export async function openDownloadDir(): Promise<void> {
  return invoke("open_download_dir");
}

export async function quitApp(): Promise<void> {
  return invoke("quit_app");
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

export interface UpdateCheckResult {
  has_update: boolean;
  latest_version: string | null;
  current_version: string;
  url: string | null;
  error?: string;
}

export async function checkUpdate(): Promise<UpdateCheckResult> {
  return invoke("check_update");
}

export interface YtdlpUpdateResult {
  has_update: boolean;
  not_installed?: boolean;
  local_version: string | null;
  latest_version: string | null;
  url: string | null;
  error?: string;
}

export async function checkYtdlpUpdate(): Promise<YtdlpUpdateResult> {
  return invoke("check_ytdlp_update");
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
