use crate::models::video_info::{Format, VideoInfo};
use serde_json::Value;

/// Parse yt-dlp --dump-json output into a VideoInfo struct.
/// Uses serde_json for most fields, with fallback extraction for edge cases.
pub fn parse_video_json(json: &str) -> Result<VideoInfo, String> {
    let root: Value = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let mut info = VideoInfo {
        id: root["id"].as_str().unwrap_or("").to_string(),
        url: root["webpage_url"]
            .as_str()
            .or_else(|| root["original_url"].as_str())
            .or_else(|| root["url"].as_str())
            .unwrap_or("")
            .to_string(),
        title: root["title"].as_str().map(|s| s.to_string()),
        description: root["description"].as_str().map(|s| s.to_string()),
        // yt-dlp reports duration as a float (seconds). as_i64() only matches
        // integer JSON values, so use as_f64() (handles ints and floats alike)
        // and truncate to whole seconds.
        duration: root["duration"].as_f64().unwrap_or(0.0) as i64,
        thumbnail: root["thumbnail"].as_str().map(|s| s.to_string()),
        uploader: root["uploader"].as_str().map(|s| s.to_string()),
        view_count: root["view_count"].as_i64().unwrap_or(0),
        like_count: root["like_count"].as_i64().unwrap_or(0),
        webpage_url: root["webpage_url"].as_str().map(|s| s.to_string()),
        formats: vec![],
        media_count: 1,
        // Download status — filled by the command layer, not by yt-dlp.
        downloaded: false,
        downloaded_at: None,
        download_path: None,
    };

    // Parse formats array
    if let Some(formats) = root["formats"].as_array() {
        for fmt_val in formats {
            if let Some(format) = parse_format(fmt_val) {
                info.formats.push(format);
            }
        }
    }

    Ok(info)
}

fn parse_format(val: &Value) -> Option<Format> {
    let format_id = val["format_id"].as_str()
        .unwrap_or_else(|| val["id"].as_str().unwrap_or("?"));
    if format_id.is_empty() || format_id == "?" {
        return None;
    }

    let vcodec = val["vcodec"].as_str().map(|s| s.to_string());
    let acodec = val["acodec"].as_str().map(|s| s.to_string());

    Some(Format {
        format_id: format_id.to_string(),
        ext: val["ext"].as_str().map(|s| s.to_string()),
        resolution: val["resolution"].as_str().map(|s| s.to_string()),
        width: val["width"].as_i64().map(|n| n as i32),
        height: val["height"].as_i64().map(|n| n as i32),
        filesize: val["filesize"].as_i64(),
        filesize_approx: val["filesize_approx"].as_i64(),
        tbr: val["tbr"].as_f64(),
        fps: val["fps"].as_f64(),
        vcodec,
        acodec,
        format_note: val["format_note"].as_str().map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let json = r#"{
            "id": "123456",
            "webpage_url": "https://x.com/user/status/123456",
            "title": "Hello World",
            "description": "A test video",
            "duration": 65.5,
            "thumbnail": "https://pbs.twimg.com/x.jpg",
            "uploader": "user",
            "view_count": 1000,
            "like_count": 42
        }"#;
        let info = parse_video_json(json).expect("should parse");
        assert_eq!(info.id, "123456");
        assert_eq!(info.title.as_deref(), Some("Hello World"));
        assert_eq!(info.description.as_deref(), Some("A test video"));
        // Float duration is truncated to whole seconds.
        assert_eq!(info.duration, 65);
        assert_eq!(info.uploader.as_deref(), Some("user"));
        assert_eq!(info.view_count, 1000);
        assert_eq!(info.like_count, 42);
        assert!(info.formats.is_empty());
        assert_eq!(info.media_count, 1);
        assert!(!info.downloaded);
    }

    #[test]
    fn test_parse_url_fallbacks() {
        // `original_url` / `url` are used when `webpage_url` is missing.
        let json = r#"{"id": "1", "original_url": "https://x.com/a/status/1"}"#;
        let info = parse_video_json(json).unwrap();
        assert_eq!(info.url, "https://x.com/a/status/1");

        let json = r#"{"id": "1", "url": "https://twitter.com/b"}"#;
        let info = parse_video_json(json).unwrap();
        assert_eq!(info.url, "https://twitter.com/b");
    }

    #[test]
    fn test_parse_duration_variants() {
        // Integer duration.
        let info = parse_video_json(r#"{"id": "1", "duration": 90}"#).unwrap();
        assert_eq!(info.duration, 90);
        // Missing duration.
        let info = parse_video_json(r#"{"id": "1"}"#).unwrap();
        assert_eq!(info.duration, 0);
    }

    #[test]
    fn test_parse_formats() {
        let json = r#"{
            "id": "1",
            "formats": [
                {"format_id": "18", "ext": "mp4", "vcodec": "h264", "acodec": "mp4a", "width": 640, "height": 360, "filesize": 1000},
                {"id": "248", "ext": "webm", "vcodec": "vp9", "acodec": "none", "resolution": "1920x1080"},
                {"ext": "mp4"}
            ]
        }"#;
        let info = parse_video_json(json).unwrap();
        assert_eq!(info.formats.len(), 2, "format without id should be skipped");
        assert_eq!(info.formats[0].format_id, "18");
        assert_eq!(info.formats[0].width, Some(640));
        assert_eq!(info.formats[0].acodec.as_deref(), Some("mp4a"));
        assert_eq!(info.formats[1].format_id, "248");
        assert_eq!(info.formats[1].resolution.as_deref(), Some("1920x1080"));
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_video_json("not json").is_err());
        assert!(parse_video_json("").is_err());
        assert!(parse_video_json("{}").is_ok());
    }

    #[test]
    fn test_parse_format_rejects_empty_id() {
        assert!(parse_format(&serde_json::json!({"ext": "mp4"})).is_none());
        assert!(parse_format(&serde_json::json!({"format_id": "?"})).is_none());
        let f = parse_format(&serde_json::json!({"format_id": "137", "vcodec": "h264"})).unwrap();
        assert_eq!(f.format_id, "137");
    }
}
