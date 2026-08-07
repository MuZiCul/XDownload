use crate::models::progress::DownloadProgress;
use regex::Regex;
use std::sync::LazyLock;

/// Matches yt-dlp's default stderr progress line:
///   [download]  50.0% of ~10.0MiB at  1.5MiB/s ETA 00:05
static PROGRESS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[download\]\s+(?P<percent>\S+%)\s+of\s+~?[\d.]+\S+\s+at\s+(?P<speed>\S+)\s+ETA\s+(?P<eta>\S+)",
    )
    .unwrap()
});

/// Matches completion line:
///   [download] 100% of 10.0MiB in 00:05 at 2.0MiB/s
static COMPLETE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[download\]\s+100%\s+of\s+[\d.]+\S+\s+in\s+(?P<eta>\S+)\s+at\s+(?P<speed>\S+)",
    )
    .unwrap()
});

/// Matches post-processing stage lines that follow the actual download, e.g.:
///   [Merger] Merging formats into "xxx.mp4"
///   [ExtractAudio] Destination: xxx.mp3
///   [EmbedThumbnail] Adding thumbnail to "xxx.mp4"
///   [Metadata] Writing metadata ...
/// These are reported so the UI can show a "merging / post-processing" state
/// instead of appearing frozen at 100%.
static POSTPROCESS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\[(Merger|ExtractAudio|EmbedThumbnail|Metadata|VideoRemuxer|FixupM4a|FixupStretched|FixupM3u8|FixupTimestamp|FixupDuration|EmbedSubtitle|EmbedInfoJson|SubtitlesConvertor|ThumbnailsConvertor|ModifyChapters|SplitChapters|MoveFiles|Mover)\].*",
    )
    .unwrap()
});

/// Parse a yt-dlp progress line into DownloadProgress.
/// Supports two formats:
/// 1. Default stderr: "[download]  XX.X% of ~XMiB at XMiB/s ETA XX:XX"
/// 2. Old --progress-template pipe-delimited format (kept for backward compat)
pub fn parse_progress_line(line: &str) -> Option<DownloadProgress> {
    // Default stderr format — in progress
    if let Some(caps) = PROGRESS_RE.captures(line) {
        return Some(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: 0,
            speed: caps.name("speed").map(|m| m.as_str().to_string()).unwrap_or_default(),
            eta: caps.name("eta").map(|m| m.as_str().to_string()).unwrap_or_default(),
            percent: caps.name("percent").map(|m| m.as_str().to_string()).unwrap_or_default(),
            status: "downloading".to_string(),
        });
    }

    // Default stderr format — completed
    if let Some(caps) = COMPLETE_RE.captures(line) {
        return Some(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: 0,
            speed: caps.name("speed").map(|m| m.as_str().to_string()).unwrap_or_default(),
            eta: caps.name("eta").map(|m| m.as_str().to_string()).unwrap_or_default(),
            percent: "100%".to_string(),
            status: "finished".to_string(),
        });
    }

    // Post-processing stages (ffmpeg merge, audio extraction, thumbnails, ...).
    // The download itself is done at this point, so report the stage name as
    // the status so the UI can show "merging / post-processing" instead of
    // appearing frozen.
    if let Some(caps) = POSTPROCESS_RE.captures(line) {
        let tag = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let status = if tag == "Merger" { "merging" } else { "postprocess" };
        return Some(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: 0,
            speed: String::new(),
            eta: String::new(),
            percent: "100%".to_string(),
            status: status.to_string(),
        });
    }

    // --progress-template pipe-delimited format (emitted to stdout once per
    // progress update, keeping HLS downloads smooth):
    //   "download:<bytes>|<total>|<speed>|<eta>|<percent>|<status>"
    // e.g. download:1234567|4567890|1.5MiB/s|00:05|45.2%|downloading
    if line.starts_with("download:") && line.contains('|') {
        let rest = &line["download:".len()..];
        let parts: Vec<&str> = rest.split('|').collect();
        if parts.len() >= 5 {
            let downloaded = parse_long_safe(parts[0]);
            let total = parse_long_safe(parts[1]);
            let speed = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
            let eta = parts.get(3).map(|s| s.to_string()).unwrap_or_default();
            let percent = parts.get(4).map(|s| s.to_string()).unwrap_or_default();
            let status = parts.get(5).map(|s| s.to_string()).unwrap_or_default();

            return Some(DownloadProgress {
                downloaded_bytes: downloaded,
                total_bytes: total,
                speed,
                eta,
                percent,
                status,
            });
        }
    }

    // Backward-compat pipe-delimited format without the "download:" prefix:
    // "bytes|total|speed|eta|percent|status"
    if line.contains('|') {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            let downloaded = parse_long_safe(parts[0]);
            let total = parse_long_safe(parts[1]);
            let speed = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
            let eta = parts.get(3).map(|s| s.to_string()).unwrap_or_default();
            let percent = parts.get(4).map(|s| s.to_string()).unwrap_or_default();
            let status = parts.get(5).map(|s| s.to_string()).unwrap_or_default();

            return Some(DownloadProgress {
                downloaded_bytes: downloaded,
                total_bytes: total,
                speed,
                eta,
                percent,
                status,
            });
        }
    }

    None
}

fn parse_long_safe(s: &str) -> i64 {
    if s.is_empty() || s == "NA" || s.eq_ignore_ascii_case("unknown") {
        return 0;
    }
    if let Ok(v) = s.parse::<f64>() {
        return v as i64;
    }
    0
}
