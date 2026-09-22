//! URL helpers shared by the download pipeline.
//!
//! Every download is keyed by the tweet **status id** (the `video_id` column of
//! the `downloads` table). yt-dlp reports its own `id` for the media it
//! extracted; that id *usually* equals the tweet id, but it occasionally points
//! at a different (related) tweet. Using the raw yt-dlp id as the history key
//! therefore produced **two rows for one video** — one failed and one
//! successful — and re-downloading could never replace the failed one.
//!
//! [`history_key`] is the single canonical rule shared by the write path
//! (queue → history), the read path (`attach_download_status`) and the
//! frontend-facing video id.

use regex::Regex;
use std::sync::OnceLock;

/// Extract the status (tweet) id from an x.com/twitter.com URL like
/// "https://x.com/user/status/1234567890123456789/video/1".
pub fn extract_status_id(url: &str) -> Option<String> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"/status/(\d+)").ok());
    re.as_ref()?
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Canonical history key for a download: the tweet status id carried by `url`,
/// falling back to `fallback` (the yt-dlp media id) when the URL has none.
///
/// Returns `None` when neither source yields a non-empty id.
pub fn history_key(url: &str, fallback: Option<&str>) -> Option<String> {
    extract_status_id(url).or_else(|| {
        fallback
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_status_id_variants() {
        assert_eq!(
            extract_status_id("https://x.com/user/status/1234567890123456789/video/1"),
            Some("1234567890123456789".to_string())
        );
        assert_eq!(
            extract_status_id("https://twitter.com/a/status/42"),
            Some("42".to_string())
        );
        assert_eq!(
            extract_status_id("https://mobile.twitter.com/a/status/7?lang=zh"),
            Some("7".to_string())
        );
        assert_eq!(extract_status_id("https://x.com/user"), None);
        assert_eq!(extract_status_id(""), None);
    }

    #[test]
    fn history_key_prefers_status_id() {
        // URL 带 status id → 永远用它，即使 fallback（yt-dlp id）不同。
        assert_eq!(
            history_key("https://x.com/a/status/900", Some("700")).as_deref(),
            Some("900")
        );
        // URL 带 status id 且 fallback 缺失 → 仍用 status id。
        assert_eq!(
            history_key("https://x.com/a/status/900", None).as_deref(),
            Some("900")
        );
    }

    #[test]
    fn history_key_falls_back_when_no_status_id() {
        assert_eq!(
            history_key("https://x.com/home", Some("700")).as_deref(),
            Some("700")
        );
        // 空 fallback 视为无值。
        assert_eq!(history_key("https://x.com/home", Some("")), None);
        assert_eq!(history_key("https://x.com/home", None), None);
    }
}
