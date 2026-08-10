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
            stage: String::new(),
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
            stage: String::new(),
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
            stage: "merge".to_string(),
        });
    }

    // --progress-template pipe-delimited format (emitted to stdout once per
    // progress update, keeping HLS downloads smooth):
    //   "download:<bytes>|<total>|<speed>|<eta>|<percent>|<status>|<acodec>|<vcodec>"
    // e.g. download:1234567|4567890|1.5MiB/s|00:05|45.2%|downloading|none|h264
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
            let acodec = parts.get(6).map(|s| s.to_string()).unwrap_or_default();
            let vcodec = parts.get(7).map(|s| s.to_string()).unwrap_or_default();

            return Some(DownloadProgress {
                downloaded_bytes: downloaded,
                total_bytes: total,
                speed,
                eta,
                percent,
                status,
                stage: stage_from_codecs(&acodec, &vcodec),
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
            let acodec = parts.get(6).map(|s| s.to_string()).unwrap_or_default();
            let vcodec = parts.get(7).map(|s| s.to_string()).unwrap_or_default();

            return Some(DownloadProgress {
                downloaded_bytes: downloaded,
                total_bytes: total,
                speed,
                eta,
                percent,
                status,
                stage: stage_from_codecs(&acodec, &vcodec),
            });
        }
    }

    None
}

/// Derive the download stage from the stream's codecs: a video-only stream
/// (`vcodec` present, `acodec` = none) is the video stage, an audio-only
/// stream the audio stage; a combined file counts as the video stage.
fn stage_from_codecs(acodec: &str, vcodec: &str) -> String {
    let has_v = !vcodec.is_empty() && vcodec != "none";
    let has_a = !acodec.is_empty() && acodec != "none";
    if has_a && !has_v {
        "audio".to_string()
    } else if has_v {
        "video".to_string()
    } else {
        String::new()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stderr_progress() {
        let p = parse_progress_line("[download]  50.0% of ~10.0MiB at  1.5MiB/s ETA 00:05")
            .expect("should parse");
        assert_eq!(p.percent, "50.0%");
        assert_eq!(p.speed, "1.5MiB/s");
        assert_eq!(p.eta, "00:05");
        assert_eq!(p.status, "downloading");
        assert!(p.stage.is_empty());
    }

    #[test]
    fn test_complete_line() {
        let p = parse_progress_line("[download] 100% of 10.0MiB in 00:05 at 2.0MiB/s")
            .expect("should parse");
        assert_eq!(p.percent, "100%");
        assert_eq!(p.status, "finished");
        assert_eq!(p.eta, "00:05");
        assert_eq!(p.speed, "2.0MiB/s");
    }

    #[test]
    fn test_postprocess_stages() {
        let merger = parse_progress_line("[Merger] Merging formats into \"xxx.mp4\"")
            .expect("should parse");
        assert_eq!(merger.status, "merging");
        assert_eq!(merger.stage, "merge");

        let extract = parse_progress_line("[ExtractAudio] Destination: xxx.mp3")
            .expect("should parse");
        assert_eq!(extract.status, "postprocess");
        assert_eq!(extract.stage, "merge");
    }

    #[test]
    fn test_pipe_template_format() {
        let p = parse_progress_line(
            "download:1234567|4567890|1.5MiB/s|00:05|45.2%|downloading|none|h264",
        )
        .expect("should parse");
        assert_eq!(p.downloaded_bytes, 1234567);
        assert_eq!(p.total_bytes, 4567890);
        assert_eq!(p.speed, "1.5MiB/s");
        assert_eq!(p.eta, "00:05");
        assert_eq!(p.percent, "45.2%");
        assert_eq!(p.status, "downloading");
        // video-only stream → video stage
        assert_eq!(p.stage, "video");
    }

    #[test]
    fn test_pipe_audio_stage() {
        let p = parse_progress_line("download:100|200|1MiB/s|00:01|50%|downloading|mp4a|none")
            .expect("should parse");
        assert_eq!(p.stage, "audio");
    }

    #[test]
    fn test_backward_compat_pipe() {
        let p = parse_progress_line("100|200|1MiB/s|00:01|50%|downloading")
            .expect("should parse");
        assert_eq!(p.downloaded_bytes, 100);
        assert_eq!(p.percent, "50%");
    }

    #[test]
    fn test_stage_from_codecs() {
        assert_eq!(stage_from_codecs("none", "h264"), "video");
        assert_eq!(stage_from_codecs("mp4a", "none"), "audio");
        assert_eq!(stage_from_codecs("mp4a", "h264"), "video");
        assert_eq!(stage_from_codecs("", ""), "");
        assert_eq!(stage_from_codecs("mp4a", "none"), "audio");
    }

    #[test]
    fn test_parse_long_safe() {
        assert_eq!(parse_long_safe("1234"), 1234);
        assert_eq!(parse_long_safe("12.9"), 12);
        assert_eq!(parse_long_safe(""), 0);
        assert_eq!(parse_long_safe("NA"), 0);
        assert_eq!(parse_long_safe("unknown"), 0);
        assert_eq!(parse_long_safe("abc"), 0);
    }

    #[test]
    fn test_unparseable_lines_return_none() {
        assert!(parse_progress_line("[download] nothing useful here").is_none());
        assert!(parse_progress_line("").is_none());
        assert!(parse_progress_line("hello world").is_none());
    }
}
