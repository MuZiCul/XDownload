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

    // Old --progress-template pipe-delimited format: "bytes|total|speed|eta|percent|status"
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
