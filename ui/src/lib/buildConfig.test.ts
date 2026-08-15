import { describe, expect, it } from "vitest";
import { buildBatchConfig, mergeSettingsIntoConfig } from "./buildConfig";
import type { AppSettings, DownloadConfig } from "./types";

const baseSettings: AppSettings = {
  download_dir: "D:/downloads",
  download_rate_limit: "1M",
  cookies_from_browser: "firefox",
};

describe("buildBatchConfig", () => {
  it("applies settings (dir / rate limit / cookies) and defaults", () => {
    const cfg = buildBatchConfig("https://x.com/a/status/1", "vid1", baseSettings);
    expect(cfg.url).toBe("https://x.com/a/status/1");
    expect(cfg.video_id).toBe("vid1");
    expect(cfg.output_dir).toBe("D:/downloads");
    expect(cfg.download_rate_limit).toBe("1M");
    expect(cfg.cookies_from_browser).toBe("firefox");
    // 批量默认格式/模板固定
    expect(cfg.format_id).toBe("bestvideo+bestaudio/best");
    expect(cfg.output_template).toBe("%(title)s.%(ext)s");
    // 批量默认无音频提取、无字幕/封面
    expect(cfg.extract_audio).toBe(false);
    expect(cfg.embed_subtitles).toBe(false);
    expect(cfg.embed_thumbnail).toBe(false);
    expect(cfg.write_thumbnail).toBe(false);
    expect(cfg.max_height).toBe(0);
    expect(cfg.socket_timeout).toBe(30);
  });

  it("falls back to defaults when settings are null", () => {
    const cfg = buildBatchConfig("https://x.com/a/status/1", null, null);
    expect(cfg.output_dir).toBe("downloads");
    expect(cfg.download_rate_limit).toBeNull();
    expect(cfg.cookies_from_browser).toBeNull();
  });

  it("keeps an empty-string dir as-is (`??` only falls back on null/undefined)", () => {
    const partial: AppSettings = { download_dir: "" };
    const cfg = buildBatchConfig("https://x.com/a/status/1", null, partial);
    // 空字符串不是 null/undefined，`?? "downloads"` 不触发，保留 ""。
    expect(cfg.output_dir).toBe("");
  });
});

describe("mergeSettingsIntoConfig", () => {
  const base: DownloadConfig = {
    url: "https://x.com/a/status/1",
    video_id: null,
    title: null,
    thumbnail: null,
    format_id: "bestvideo+bestaudio/best",
    output_dir: "downloads",
    output_template: "%(title)s.%(ext)s",
    extract_audio: false,
    embed_subtitles: false,
    embed_thumbnail: false,
    write_thumbnail: false,
    proxy: null,
    socket_timeout: 30,
    download_rate_limit: null,
    cookies_from_browser: null,
    max_height: 0,
    download_archive: null,
  };

  it("overlays fresh settings onto the config", () => {
    const merged = mergeSettingsIntoConfig(base, baseSettings);
    expect(merged.output_dir).toBe("D:/downloads");
    expect(merged.download_rate_limit).toBe("1M");
    expect(merged.cookies_from_browser).toBe("firefox");
  });

  it("keeps the rest of the config untouched", () => {
    const withFormat: DownloadConfig = { ...base, format_id: "137+140" };
    const merged = mergeSettingsIntoConfig(withFormat, baseSettings);
    expect(merged.format_id).toBe("137+140");
    expect(merged.url).toBe(base.url);
    expect(merged.max_height).toBe(0);
  });

  it("does not mutate the input config", () => {
    const snapshot = JSON.stringify(base);
    mergeSettingsIntoConfig(base, baseSettings);
    expect(JSON.stringify(base)).toBe(snapshot);
  });

  it("falls back to defaults when settings are null", () => {
    const merged = mergeSettingsIntoConfig(base, null);
    expect(merged.output_dir).toBe("downloads");
    expect(merged.download_rate_limit).toBeNull();
    expect(merged.cookies_from_browser).toBeNull();
  });
});
