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
        duration: root["duration"].as_i64().unwrap_or(0),
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
