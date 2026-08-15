import type { AppSettings, DownloadConfig } from "./types";

/**
 * Pure config-building helpers, extracted from `downloadStore.ts` /
 * `DownloadPage.tsx` so the shared download-config logic can be unit-tested
 * without mocking the Tauri bridge.
 *
 * Invariants (kept identical to the original implementations):
 * - `buildBatchConfig` returns the batch-style config used by the history
 *   page, duplicate-confirm modal and bookmarks list.
 * - `mergeSettingsIntoConfig` overlays live settings onto an existing config
 *   (the former `buildLatestConfig` merge step in DownloadPage).
 */

/** Build a batch-style DownloadConfig from settings. Shared by the history
 *  page, duplicate-confirm modal and the bookmarks list, so every download
 *  path honours the same settings. */
export function buildBatchConfig(
  u: string,
  videoId: string | null,
  s: AppSettings | null
): DownloadConfig {
  return {
    url: u,
    video_id: videoId,
    title: null,
    thumbnail: null,
    format_id: "bestvideo+bestaudio/best",
    output_dir: s?.download_dir ?? "downloads",
    output_template: "%(title)s.%(ext)s",
    extract_audio: false,
    embed_subtitles: false,
    embed_thumbnail: false,
    write_thumbnail: false,
    proxy: null,
    socket_timeout: 30,
    download_rate_limit: s?.download_rate_limit ?? null,
    cookies_from_browser: s?.cookies_from_browser ?? null,
    max_height: 0,
    download_archive: null,
  };
}

/**
 * Overlay live settings onto an existing config (used right before a single
 * download starts, so output_dir / rate limit / cookies are always fresh).
 * Returns a new object; the input is not mutated.
 */
export function mergeSettingsIntoConfig(
  config: DownloadConfig,
  s: AppSettings | null
): DownloadConfig {
  return {
    ...config,
    output_dir: s?.download_dir ?? "downloads",
    download_rate_limit: s?.download_rate_limit ?? null,
    cookies_from_browser: s?.cookies_from_browser ?? null,
  };
}
