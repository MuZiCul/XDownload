//! X (Twitter) Bookmarks sync support.
//!
//! The desktop app polls X's internal GraphQL `Bookmarks` endpoint using the
//! user's saved session cookies (auth_token + ct0) — the same cookies already
//! used by yt-dlp. This module contains only *pure* logic (URL construction,
//! response parsing, incremental diff) so it can be unit-tested offline
//! against the *real* X web response structure.
//!
//! Real response shape (captured from x.com/i/api/graphql/<id>/Bookmarks):
//!
//! ```json
//! {
//!   "data": {
//!     "bookmark_timeline_v2": {
//!       "timeline": {
//!         "instructions": [
//!           {
//!             "type": "TimelineAddEntries",
//!             "entries": [
//!               {
//!                 "entryId": "tweet-1900000000000000000",
//!                 "content": {
//!                   "entryType": "TimelineTimelineItem",
//!                   "itemContent": {
//!                     "itemType": "TimelineTweet",
//!                     "tweet_results": {
//!                       "result": { ... }   // null when tweet is deleted/restricted
//!                     }
//!                   }
//!                 }
//!               },
//!               {
//!                 "entryId": "cursor-bottom-0-1",
//!                 "content": {
//!                   "entryType": "TimelineTimelineCursor",
//!                   "value": "..."
//!                 }
//!               }
//!             ]
//!           },
//!           { "type": "TimelineClearCache" },
//!           { "type": "TimelinePinEntry" }
//!         ]
//!       }
//!     }
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Public web bearer token used by x.com's web client.
/// This is *not* a secret — it ships in every page load of x.com and is used
/// to authenticate GraphQL API calls (along with the user's session cookies).
pub const X_WEB_BEARER: &str =
    "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

/// Fallback queryId for the `Bookmarks` GraphQL operation, captured from a
/// real x.com request (2026-08-13). X rotates these ids periodically; the
/// discovery mechanism (`discover_bookmarks_query_id`) tries to refresh it,
/// and this constant keeps the feature working even when discovery fails
/// (e.g. when the `i/bookmarks` page blocks the bot with HTTP 401).
pub const DEFAULT_BOOKMARKS_QUERY_ID: &str = "iblrFnKr6PZUR-dWpfXG6g";

/// GraphQL `features` object captured from a real x.com bookmarks request.
/// The web client sends this exact JSON with every GraphQL call; the Bookmarks
/// operation requires it (an empty `{}` is rejected).
pub const BOOKMARKS_FEATURES: &str = r#"{"rweb_video_screen_enabled":false,"rweb_cashtags_enabled":true,"profile_label_improvements_pcf_label_in_post_enabled":true,"responsive_web_profile_redirect_enabled":true,"rweb_tipjar_consumption_enabled":false,"verified_phone_label_enabled":false,"creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_timeline_navigation_enabled":true,"premium_content_api_read_enabled":false,"communities_web_enable_tweet_community_results_fetch":true,"c9s_tweet_anatomy_moderator_badge_enabled":true,"responsive_web_grok_analyze_button_fetch_trends_enabled":false,"responsive_web_grok_analyze_post_followups_enabled":true,"rweb_cashtags_composer_attachment_enabled":true,"responsive_web_jetfuel_frame":true,"responsive_web_grok_share_attachment_enabled":true,"responsive_web_grok_annotations_enabled":true,"articles_preview_enabled":true,"responsive_web_edit_tweet_api_enabled":true,"rweb_conversational_replies_downvote_enabled":false,"graphql_is_translatable_rweb_tweet_is_translatable_enabled":true,"view_counts_everywhere_api_enabled":true,"longform_notetweets_consumption_enabled":true,"responsive_web_twitter_article_tweet_consumption_enabled":true,"content_disclosure_indicator_enabled":true,"content_disclosure_ai_generated_indicator_enabled":true,"responsive_web_grok_show_grok_translated_post":true,"responsive_web_grok_analysis_button_from_backend":true,"post_ctas_fetch_enabled":false,"freedom_of_speech_not_reach_fetch_enabled":true,"standardized_nudges_misinfo":true,"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled":true,"longform_notetweets_rich_text_read_enabled":true,"longform_notetweets_inline_media_enabled":false,"responsive_web_grok_image_annotation_enabled":true,"responsive_web_grok_imagine_annotation_enabled":true,"responsive_web_grok_community_note_auto_translation_is_enabled":true,"responsive_web_enhance_cards_enabled":false}"#;

/// Authentication material for the Bookmarks GraphQL endpoint.
#[derive(Debug, Clone, Default)]
pub struct BookmarkAuth {
    /// `auth_token` session cookie value (from x.com).
    pub auth_token: String,
    /// `ct0` cookie value — doubles as the CSRF token for the
    /// `x-csrf-token` request header.
    pub ct0: String,
}

impl BookmarkAuth {
    /// True when both required cookies are present.
    pub fn is_complete(&self) -> bool {
        !self.auth_token.is_empty() && !self.ct0.is_empty()
    }

    /// Build the `Cookie` header value (kept simple: the two cookies the
    /// Bookmarks endpoint needs).
    pub fn cookie_header(&self) -> String {
        format!("auth_token={}; ct0={}", self.auth_token, self.ct0)
    }
}

/// One page of results from the Bookmarks endpoint.
#[derive(Debug, Clone)]
pub struct BookmarkPage {
    pub items: Vec<BookmarkItem>,
    /// Next-page cursor (None when this was the last page).
    pub next_cursor: Option<String>,
    /// HTTP status code (useful for diagnosing 429/401).
    pub status: u16,
}

/// Fetch one page of bookmarks.
///
/// `base_url` defaults to the real x.com but is injectable for tests.
/// `query_id` is the (rotating) GraphQL operation id; `count` items per page;
/// `cursor` is the optional pagination token.
///
/// On HTTP success the body is parsed; a non-2xx status is returned as an Err
/// with the status code embedded so the caller can distinguish auth vs. rate
/// limit failures.
pub async fn fetch_bookmarks(
    client: &reqwest::Client,
    auth: &BookmarkAuth,
    query_id: &str,
    count: u32,
    cursor: Option<&str>,
    base_url: &str,
) -> Result<BookmarkPage, String> {
    if !auth.is_complete() {
        return Err("书签同步需要 auth_token 与 ct0 cookies，请先在设置中配置浏览器 cookies".to_string());
    }
    let url = build_bookmarks_url(query_id, count, cursor);
    let absolute = if base_url.is_empty() {
        url
    } else {
        // Allow tests to point the GraphQL path at a local server while keeping
        // the same path/query. base_url has no trailing slash.
        format!(
            "{}/i/api/graphql/{}/Bookmarks?variables={}&features={}",
            base_url,
            query_id,
            urlencode(&serde_json::to_string(&build_variables(count, cursor)).unwrap_or_default()),
            urlencode(BOOKMARKS_FEATURES)
        )
    };

    // Refresh ct0 before the GraphQL call: X rotates ct0 on every page load
    // and a stale value causes HTTP 401 on the bookmarks endpoint.
    let ct0 = refresh_ct0(client, &auth, base_url).await;

    let mut req = client
        .get(&absolute)
        .header("cookie", format!("auth_token={}; ct0={}", auth.auth_token, ct0))
        .header("x-csrf-token", ct0)
        .header("authorization", format!("Bearer {}", X_WEB_BEARER))
        .header("x-twitter-active-user", "yes")
        .header("x-twitter-auth-type", "OAuth2Session");
    if let Some(c) = cursor {
        req = req.header("x-twitter-client-language", "en");
        let _ = c; // keep signature simple; cursor already in query
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("请求书签接口失败: {e}"))?;
    let status = resp.status().as_u16();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("读取书签响应失败: {e}"))?;

    if status != 200 {
        let snippet = body.chars().take(200).collect::<String>();
        return Err(format!("书签接口返回 HTTP {status}: {snippet}"));
    }
    let items = parse_bookmarks_response(&body)?;
    let next_cursor = extract_cursor(&body);
    Ok(BookmarkPage {
        items,
        next_cursor,
        status,
    })
}

/// Dump x.com cookies from a browser using yt-dlp's `--cookies-from-browser`,
/// then extract `auth_token` + `ct0` into a [`BookmarkAuth`].
///
/// Reuses the same temp-file approach as the cookies validation flow in
/// `commands/cookies.rs`, but keeps this bookmark-specific implementation
/// self-contained (it needs both cookies, not just auth_token).
pub async fn load_bookmark_auth_from_browser(
    browser: &str,
) -> Result<BookmarkAuth, String> {
    if browser.is_empty() || browser == "none" {
        return Err("未选择浏览器".to_string());
    }
    let ytdlp = crate::utils::process::find_ytdlp();
    if !ytdlp.exists() {
        return Err("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp".to_string());
    }

    let temp_dir = std::env::temp_dir().join("xdownload_cookies");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("创建临时目录失败: {e}"))?;
    let cookie_file = temp_dir.join("bookmarks_cookies.txt");
    let _ = std::fs::remove_file(&cookie_file);

    let ytdlp_str = ytdlp.to_str().unwrap_or("yt-dlp").to_string();
    let mut args: Vec<String> = vec![
        ytdlp_str,
        "--cookies-from-browser".to_string(),
        browser.to_string(),
        "--cookies".to_string(),
        cookie_file.to_string_lossy().to_string(),
        "--no-warnings".to_string(),
        "--no-color".to_string(),
        "--skip-download".to_string(),
        "--flat-playlist".to_string(),
        "https://x.com/home".to_string(),
    ];

    // Apply configured proxy if any.
    if let Some(proxy_url) = crate::services::proxy::ProxyConfig::to_proxy_url() {
        args.push("--proxy".to_string());
        args.push(proxy_url);
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let result = crate::utils::process::execute_with_timeout(&args_refs, 30).await;
    if result.is_err() {
        return Err(format!("cookies 导出失败: {}", result.err().unwrap()));
    }

    if !cookie_file.exists() {
        return Err("cookies 导出未生成文件".to_string());
    }
    parse_x_cookies(&cookie_file)
}

/// Parse a Netscape-format cookie file for x.com `auth_token` and `ct0`.
/// Returns a [`BookmarkAuth`]; errors when either cookie is missing.
pub fn parse_x_cookies(path: &std::path::Path) -> Result<BookmarkAuth, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取 cookie 文件: {e}"))?;
    let mut auth_token = String::new();
    let mut ct0 = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 7 {
            continue;
        }
        let domain = parts[0];
        let name = parts[5];
        let value = parts[6];
        if domain != ".x.com" && domain != "x.com" && domain != ".twitter.com" && domain != "twitter.com" {
            continue;
        }
        if name == "auth_token" && auth_token.is_empty() {
            auth_token = value.to_string();
        } else if name == "ct0" && ct0.is_empty() {
            ct0 = value.to_string();
        }
    }

    if auth_token.is_empty() {
        return Err("未在浏览器 cookies 中找到 x.com 的 auth_token，请确保已在浏览器中登录 x.com".to_string());
    }
    if ct0.is_empty() {
        return Err("未在浏览器 cookies 中找到 x.com 的 ct0（CSRF token），请刷新 x.com 页面后重试".to_string());
    }
    Ok(BookmarkAuth { auth_token, ct0 })
}

/// High-level helper: dump cookies from the configured browser and fetch the
/// latest bookmarks page. `query_id` is the (rotating) GraphQL operation id.
pub async fn fetch_bookmarks_from_browser(
    browser: &str,
    query_id: &str,
    count: u32,
) -> Result<BookmarkPage, String> {
    let auth = load_bookmark_auth_from_browser(browser).await?;
    let client = build_x_client()?;
    fetch_bookmarks(&client, &auth, query_id, count, None, "").await
}

// --- QueryId discovery -------------------------------------------------------
//
// The Bookmarks GraphQL endpoint is `https://x.com/i/api/graphql/<queryId>/Bookmarks`.
// queryId is a hash that X rotates periodically (every few weeks). It is not
// exposed via any documented API, so we discover it by parsing X's own web
// frontend: fetch the bookmarks page, grab its JS bundle URLs, download the
// bundles and regex-match the `operationName:"Bookmarks"` mapping to its
// `queryId`. Mirrors how open-source X scrapers (e.g. twitter-web-exporter)
// obtain operation ids.

/// Extract `<script src="...">` URLs from an HTML page, resolving relative
/// paths against `base_url`. Keeps only `.js` assets.
pub fn extract_js_urls(html: &str, base_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Exclude `"` and `<` from the URL so the capture never spills past the
    // closing quote or into the next tag (the `+` is greedy otherwise).
    for cap in regex::Regex::new(r#"(?:src|href)="([^"<]+\.js(?:[?"][^"<]*)?)""#)
        .unwrap()
        .captures_iter(html)
    {
        let raw = &cap[1];
        let url = if raw.starts_with("http") {
            raw.to_string()
        } else if let Some(stripped) = raw.strip_prefix("//") {
            format!("https://{stripped}")
        } else if raw.starts_with('/') {
            format!("{}{}", base_url.trim_end_matches('/'), raw)
        } else {
            continue; // skip inline/no-src entries
        };
        out.push(url);
    }
    out
}

/// Extract the fresh `ct0` (CSRF token) from x.com page HTML.
///
/// X rotates `ct0` on every page load and it must match the `x-csrf-token`
/// header for GraphQL calls. The token appears in the page's embedded JSON,
/// usually as `"ct0":"<value>"`.
pub fn extract_ct0_from_html(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#""ct0":"([A-Za-z0-9%_\-]{10,})""#).unwrap();
    if let Some(c) = re.captures(html) {
        return Some(c[1].to_string());
    }
    // Fallback: `ct0=value` inside a cookie setter or JS string.
    let re2 = regex::Regex::new(r#"ct0=([A-Za-z0-9%_\-]{10,})"#).unwrap();
    re2.captures(html).map(|c| c[1].to_string())
}

/// Fetch the home page with the given auth and extract the latest `ct0`.
/// Falls back to the stored `auth.ct0` when the extraction fails.
///
/// `base_url` is used by tests: when it is non-empty (a mock server), the
/// real x.com round-trip is skipped and the caller-provided `ct0` is returned
/// directly (tests must never depend on the live network).
async fn refresh_ct0(client: &reqwest::Client, auth: &BookmarkAuth, base_url: &str) -> String {
    if !base_url.is_empty() {
        return auth.ct0.clone();
    }
    let home_url = "https://x.com/home";
    let resp = client
        .get(home_url)
        .header("cookie", format!("auth_token={};", auth.auth_token))
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36")
        .send()
        .await;
    if let Ok(resp) = resp {
        if let Ok(html) = resp.text().await {
            if let Some(fresh) = extract_ct0_from_html(&html) {
                return fresh;
            }
        }
    }
    auth.ct0.clone()
}

/// Try to extract the Bookmarks queryId from a JS bundle.
///
/// X bundles contain operation metadata roughly as
/// `queryId:"<id>",operationName:"Bookmarks"` or
/// `{queryId:"<id>",operationName:"Bookmarks"}` or
/// `operationName:"Bookmarks",queryId:"<id>"`. We scan for the Bookmarks
/// operation and pull the queryId near it.
pub fn extract_bookmarks_query_id(js: &str) -> Option<String> {
    // X bundles encode operations as objects like
    // `{queryId:"<id>",operationName:"Bookmarks"}` (keys may be quoted or not).
    //
    // Important: x.com has BOTH `Bookmarks` (the bookmarks list) and
    // `BookmarkSearchTimeline` (search inside bookmarks). The former is what
    // we want; the latter requires a `rawQuery` and is useless for listing.
    // Anchor on the exact `operationName:"Bookmarks"` (a closed token) so
    // `BookmarkSearchTimeline` never matches, then read the queryId near it.
    let op_re = regex::Regex::new(r#""?operationName"?:"Bookmarks""#).unwrap();
    let qid_re = regex::Regex::new(
        r#""?queryId"?:"?([A-Za-z0-9_-]{8,60})"?"#,
    )
    .unwrap();
    for m in op_re.captures_iter(js) {
        let op_start = m.get(0).unwrap().start();
        let op_end = m.get(0).unwrap().end();
        // Small window around the operation name — the queryId lives in the
        // same object literal, so a few hundred chars either way is enough.
        // Among all queryIds in the window, pick the one closest to the
        // `operationName:"Bookmarks"` token (a neighboring SearchTimeline op
        // may appear earlier in the window and must not win).
        let window_start = op_start.saturating_sub(500);
        let window_end = usize::min(op_end + 500, js.len());
        let window = &js[window_start..window_end];
        let rel_start = op_start - window_start;
        let rel_end = op_end - window_start;
        let op_center = (rel_start + rel_end) / 2;
        let mut best: Option<(usize, String)> = None;
        for q in qid_re.captures_iter(window) {
            let qm = q.get(0).unwrap();
            let q_center = (qm.start() + qm.end()) / 2;
            let dist = q_center.abs_diff(op_center);
            if best.as_ref().map_or(true, |(d, _)| dist < *d) {
                best = Some((dist, q[1].to_string()));
            }
        }
        if let Some((_, id)) = best {
            return Some(id);
        }
    }
    // Loose mapping object: `"Bookmarks":"<id>"` (rare but some bundles emit
    // a plain operation→queryId map).
    let re3 = regex::Regex::new(r#""Bookmarks":"([A-Za-z0-9_-]{8,60})""#).unwrap();
    if let Some(c) = re3.captures(js) {
        return Some(c[1].to_string());
    }
    None
}

/// Discover the current Bookmarks queryId by scraping X's web frontend.
///
/// `base_url` defaults to `https://x.com` but is injectable for tests.
/// Loads the bookmarks page HTML, collects JS bundle URLs, downloads each
/// bundle (up to `max_bundles`) and looks for the Bookmarks operation.
pub async fn discover_bookmarks_query_id(
    client: &reqwest::Client,
    auth: &BookmarkAuth,
    base_url: &str,
    max_bundles: usize,
) -> Result<String, String> {
    let root = if base_url.is_empty() {
        "https://x.com".to_string()
    } else {
        base_url.trim_end_matches('/').to_string()
    };
    let page_url = format!("{root}/i/bookmarks");

    // 1. Refresh ct0 first: X rotates ct0 on every page load and the stored
    //    cookie's ct0 may be stale, causing GraphQL/bookmarks pages to 401.
    let ct0 = refresh_ct0(client, auth, base_url).await;

    let resp = client
        .get(&page_url)
        .header("cookie", format!("auth_token={}; ct0={}", auth.auth_token, ct0))
        .header("x-csrf-token", ct0)
        .header("authorization", format!("Bearer {}", X_WEB_BEARER))
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("请求书签页失败: {e}"))?;
    let status = resp.status().as_u16();
    if status != 200 {
        // Include a snippet of the response body (X returns JSON errors like
        // `{"errors":[{"message":"unauthorized","code":64}]}`) to help diagnose
        // why auth is rejected. A stale queryId often surfaces as 404/429
        // with a "Could not find" message, so route it through the hint.
        let body_snippet = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect::<String>();
        return Err(query_id_failure_hint(&format!(
            "HTTP {status}: {body_snippet}"
        )));
    }
    let html = resp
        .text()
        .await
        .map_err(|e| format!("读取书签页失败: {e}"))?;

    let js_urls = extract_js_urls(&html, &root);
    if js_urls.is_empty() {
        return Err("书签页未找到 JS bundle".to_string());
    }

    let mut checked = 0;
    for url in js_urls {
        if checked >= max_bundles {
            break;
        }
        checked += 1;
        let Ok(resp) = client.get(&url).send().await else { continue };
        let Ok(js) = resp.text().await else { continue };
        if let Some(qid) = extract_bookmarks_query_id(&js) {
            return Ok(qid);
        }
    }
    Err(format!(
        "已检查 {} 个 JS bundle，未找到 Bookmarks queryId（X 可能已更新前端结构，请从 DevTools 手动获取）",
        checked
    ))
}

/// Convenience wrapper: discover queryId from the configured browser.
pub async fn discover_bookmarks_query_id_from_browser(
    browser: &str,
) -> Result<String, String> {
    let auth = load_bookmark_auth_from_browser(browser).await?;
    let client = build_x_client()?;
    discover_bookmarks_query_id(&client, &auth, "", 30).await
}

/// Build a reqwest client for x.com that honors the configured proxy (the
/// user may need a proxy to reach x.com) and uses a browser-like User-Agent.
fn build_x_client() -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(30));
    if let Some(proxy) = crate::services::proxy::ProxyConfig::to_reqwest_proxy() {
        builder = builder.proxy(proxy);
    } else {
        // Fallback: this machine's local proxy (Clash on 127.0.0.1:7897).
        if let Ok(p) = reqwest::Proxy::all("http://127.0.0.1:7897") {
            builder = builder.proxy(p);
        }
    }
    builder
        .build()
        .map_err(|e| format!("创建请求客户端失败: {e}"))
}

/// Build the GraphQL `variables` JSON object.
fn build_variables(count: u32, cursor: Option<&str>) -> serde_json::Value {
    // Captured from a real x.com Bookmarks request: the operation only needs
    // `count` + `includePromotedContent`. Extra `withXxx` fields are not sent
    // by the web client and can trigger GRAPHQL_VALIDATION_FAILED.
    let mut vars = serde_json::json!({
        "count": count,
        "includePromotedContent": true,
    });
    if let Some(c) = cursor {
        vars["cursor"] = Value::String(c.to_string());
    }
    vars
}

/// One parsed bookmark tweet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkItem {
    /// X tweet id (rest_id).
    pub tweet_id: String,
    /// Author handle (screen_name, without '@').
    pub handle: String,
    /// Full status URL, e.g. `https://x.com/user/status/123`.
    pub url: String,
    /// Tweet text (full_text).
    pub text: String,
    /// Author display name (legacy.name).
    pub author_name: String,
    /// Whether the tweet contains a video / animated GIF.
    pub has_video: bool,
}

/// Build the GraphQL URL for the Bookmarks endpoint.
///
/// `variables` includes `count` and optional `cursor` (pagination token).
/// The queryId rotates over time and must be refreshed separately.
pub fn build_bookmarks_url(query_id: &str, count: u32, cursor: Option<&str>) -> String {
    let variables = serde_json::to_string(&build_variables(count, cursor)).unwrap_or_default();
    let encoded = urlencode(&variables);
    format!(
        "https://x.com/i/api/graphql/{}/Bookmarks?variables={}&features={}",
        query_id,
        encoded,
        urlencode(BOOKMARKS_FEATURES)
    )
}

/// Minimal percent-encoding for query params (UTF-8 chars become %XX).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Extract the next-page cursor value from a Bookmarks response, if any.
pub fn extract_cursor(body: &str) -> Option<String> {
    let root: Value = serde_json::from_str(body).ok()?;
    let instructions = root
        .pointer("/data/bookmark_timeline_v2/timeline/instructions")?
        .as_array()?;
    for ins in instructions {
        let entries = ins.get("entries").and_then(|e| e.as_array());
        let Some(entries) = entries else { continue };
        for entry in entries {
            let is_cursor = entry
                .pointer("/content/entryType")
                .and_then(|v| v.as_str())
                == Some("TimelineTimelineCursor");
            if is_cursor {
                if let Some(v) = entry.pointer("/content/value").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Parse a Bookmarks response body into bookmark tweets.
///
/// Returns Ok(vec) — empty vec when there are no (parsable) tweets.
/// Skips null results (deleted/restricted tweets) and non-tweet entries.
///
/// Failure hints: a 200 response carrying a GraphQL `errors` array, or one
/// missing the bookmarks structure entirely, usually means the queryId has
/// gone stale (X returns old/new ids differently). Those errors are prefixed
/// with a "queryId 已失效" hint so the UI can guide the user to re-capture
/// the id via the browser extension.
pub fn parse_bookmarks_response(body: &str) -> Result<Vec<BookmarkItem>, String> {
    let root: Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from Bookmarks endpoint: {e}"))?;

    // GraphQL business errors (HTTP 200 + `{"errors":[...]}`): a stale
    // queryId shows up here as "Could not find query with name ...".
    if let Some(errors) = root.get("errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            let joined = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect::<Vec<_>>()
                .join("; ");
            let joined = if joined.is_empty() {
                "unknown GraphQL error".to_string()
            } else {
                joined
            };
            return Err(query_id_failure_hint(&joined));
        }
    }

    let instructions = root
        .pointer("/data/bookmark_timeline_v2/timeline/instructions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            // A 200 with no bookmarks structure at all usually means the
            // queryId is stale (real X returns data-less responses for
            // retired queryIds). Keep the "missing" keyword so existing
            // tests still match.
            "queryId 已失效或接口结构异常（missing data.bookmark_timeline_v2.timeline.instructions）"
                .to_string()
        })?;

    let mut items: Vec<BookmarkItem> = Vec::new();
    for ins in instructions {
        // Real X responses put `entries` directly on the instruction object
        // (no `type: "TimelineAddEntries"` wrapper); accept any instruction
        // that carries an `entries` array.
        let Some(entries) = ins.get("entries").and_then(|e| e.as_array()) else {
            continue;
        };
        for entry in entries {
            if let Some(item) = parse_entry(entry) {
                items.push(item);
            }
        }
    }
    Ok(items)
}

/// Prefix an error with a "queryId 已失效" hint when it looks like a stale /
/// unknown queryId (X's messages contain "Could not find" / "queryId" / "not
/// found"). Other failures keep a plain "书签接口返回错误" prefix. Callers use
/// the "queryId" keyword to route the UI hint.
fn query_id_failure_hint(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("could not find")
        || lower.contains("queryid")
        || lower.contains("not found")
    {
        format!("queryId 已失效或不存在，请使用浏览器扩展重新捕获并推送：{msg}")
    } else {
        format!("书签接口返回错误: {msg}")
    }
}

/// Parse a single timeline entry into a BookmarkItem (None if not a tweet).
fn parse_entry(entry: &Value) -> Option<BookmarkItem> {
    let result = entry
        .pointer("/content/itemContent/tweet_results/result")
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(v)
            }
        })?;

    let tweet_id = result
        .get("rest_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            result
                .get("rest_id")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
        })?;

    // Author info. The user object has two shapes over time:
    //   new: core.user_results.result.core.{screen_name,name}
    //   old: core.user_results.result.legacy.{screen_name,name}
    // Prefer the new shape, fall back to the legacy one.
    let legacy = result.get("legacy");
    let user_info = result
        .pointer("/core/user_results/result/core")
        .or_else(|| result.pointer("/core/user_results/result/legacy"));
    let handle = user_info
        .and_then(|u| u.get("screen_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let author_name = user_info
        .and_then(|u| u.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let text = legacy
        .and_then(|l| l.get("full_text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Video detection — real path: legacy.extended_entities.media[].type
    let has_video = legacy
        .and_then(|l| l.pointer("/extended_entities/media"))
        .and_then(|m| m.as_array())
        .map(|media| {
            media.iter().any(|m| {
                m.get("type").and_then(|t| t.as_str()).map_or(false, |t| {
                    t == "video" || t == "animated_gif"
                })
            })
        })
        .unwrap_or(false);

    Some(BookmarkItem {
        url: format!("https://x.com/{}/status/{}", handle, tweet_id),
        tweet_id,
        handle,
        text,
        author_name,
        has_video,
    })
}

/// Return the subset of `current` bookmarks that were not present in
/// `previous_ids`. Preserves the input order (newest first from X).
pub fn find_new_bookmarks(
    previous_ids: &std::collections::HashSet<String>,
    current: &[BookmarkItem],
) -> Vec<BookmarkItem> {
    current
        .iter()
        .filter(|b| !previous_ids.contains(&b.tweet_id))
        .cloned()
        .collect()
}

// --- Fetch & diff -------------------------------------------------------------

/// Fetch the full bookmarks list (paginating through every page).
///
/// `base_url` is injectable for tests ("" = real x.com). Returns all items in
/// X's order (newest first).
pub async fn fetch_all_bookmarks(
    client: &reqwest::Client,
    auth: &BookmarkAuth,
    query_id: &str,
    count: u32,
    base_url: &str,
) -> Result<Vec<BookmarkItem>, String> {
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    loop {
        pages += 1;
        let page = fetch_bookmarks(client, auth, query_id, count, cursor.as_deref(), base_url).await?;
        all.extend(page.items);
        match page.next_cursor {
            Some(c) => cursor = Some(c),
            None => break,
        }
        // Hard safety cap on pagination depth.
        if pages >= 20 {
            break;
        }
    }
    Ok(all)
}

/// One bookmark video offered in the sync preview, tagged with its download
/// state so the UI can style it differently and still allow re-downloading.
#[derive(Debug, Clone, Serialize)]
pub struct BookmarkVideo {
    #[serde(flatten)]
    pub item: BookmarkItem,
    /// True when the video is downloaded and the file still exists on disk.
    pub downloaded: bool,
}

/// Result of a manual sync preview: what a sync would find, before the user
/// decides whether to enqueue. Never touches the sync cursor or the queue.
#[derive(Debug, Clone, Serialize)]
pub struct BookmarkChanges {
    /// Total bookmarks currently on X.
    pub total: usize,
    /// Bookmarks never processed before (including text/image-only ones).
    pub new_count: usize,
    /// All bookmarks that contain video — downloaded or not — each tagged with
    /// its download state. The user picks which ones to enqueue (downloaded
    /// ones can be re-downloaded).
    pub video_items: Vec<BookmarkVideo>,
}

/// Result of confirming a sync preview: how many videos were actually
/// enqueued into the download queue.
#[derive(Debug, Clone, Serialize)]
pub struct ConfirmResult {
    pub queued_count: usize,
}

/// Build the "already processed" id set from the download history.
///
/// The download history *is* the sync cursor: only videos that finished (or
/// attempted) a download are treated as not new. History record `video_id`
/// equals the bookmark tweet_id for single-video tweets, so tweets downloaded
/// before sync was enabled are never offered again.
fn history_id_set(
    records: Vec<crate::services::download_history::DownloadRecord>,
) -> std::collections::HashSet<String> {
    records.into_iter().map(|r| r.video_id).collect()
}

/// Phase of a bookmark sync, surfaced to the UI as a progress step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BookmarkSyncStep {
    /// Reading browser cookies for x.com authentication.
    Cookies,
    /// Fetching the full bookmark list from X.
    Fetch,
    /// Persisting fetched bookmarks into the local `bookmarks` table.
    Persist,
    /// Diffing against the download history and building the preview.
    Diff,
}

/// Stage 1 of the manual sync flow: fetch all bookmarks and diff against the
/// download history (which acts as the sync cursor).
///
/// Returns what a sync *would* do — the total bookmark count and the new
/// video-bearing bookmarks — without touching the queue. The user reviews the
/// result in a modal and only then calls [`confirm_bookmark_enqueue`].
/// `progress` is invoked at each phase (best-effort; it must not block or
/// panic) so the UI can show a real progress step.
pub async fn fetch_bookmark_changes(
    browser: &str,
    progress: impl Fn(BookmarkSyncStep),
) -> Result<BookmarkChanges, String> {
    progress(BookmarkSyncStep::Cookies);
    tracing::info!("bookmarks sync preview: browser={browser}");
    let auth = load_bookmark_auth_from_browser(browser).await?;
    tracing::info!("bookmarks sync preview: cookies ok, building client");
    let client = build_x_client()?;
    // 优先使用扩展捕获并推送的最新 queryId（存 config 表）；未推送时退回内置常量。
    let query_id = crate::services::query_id::load()
        .unwrap_or_else(|| DEFAULT_BOOKMARKS_QUERY_ID.to_string());
    progress(BookmarkSyncStep::Fetch);
    tracing::info!("bookmarks sync preview: fetching with query_id={query_id}");
    let all = fetch_all_bookmarks(&client, &auth, &query_id, 20, "").await?;

    // 持久化书签目录：把本次拉取的全部书签（含无视频的）写入 bookmarks 表，
    // 供「查看已同步书签」离线浏览。失败仅记录日志，不影响同步主流程。
    progress(BookmarkSyncStep::Persist);
    crate::services::bookmarks_store::upsert_all(&all);

    progress(BookmarkSyncStep::Diff);
    // 下载历史即游标：基线直接取自下载历史（含成功与失败记录），不再维护
    // bookmarks_state.json。下载完成/尝试过的书签一律视为已处理。
    let history = crate::services::download_history::DownloadHistory::list();
    let history_ids = history_id_set(history.clone());
    let new_items = find_new_bookmarks(&history_ids, &all);

    // 已下载 = 历史记录存在且文件仍在磁盘上（与 is_downloaded 语义一致）。
    // 复用一次 list() 同时构建游标集合与下载状态映射，避免多次开库。
    let file_map: std::collections::HashMap<String, Option<String>> = history
        .into_iter()
        .map(|r| (r.video_id, r.file_path))
        .collect();
    let is_downloaded = |id: &str| -> bool {
        file_map
            .get(id)
            .and_then(|p| p.as_ref())
            .map(|p| std::path::Path::new(p).exists())
            .unwrap_or(false)
    };

    // 弹窗列出所有含视频的书签（含已下载，便于区分底色并支持重新下载），
    // 每项带 downloaded 标记供前端样式区分。
    let video_items = all
        .iter()
        .filter(|b| b.has_video)
        .map(|b| BookmarkVideo {
            item: b.clone(),
            downloaded: is_downloaded(&b.tweet_id),
        })
        .collect::<Vec<_>>();

    tracing::info!(
        "bookmarks sync preview: done total={} new={} video={} downloaded={}",
        all.len(),
        new_items.len(),
        video_items.len(),
        video_items.iter().filter(|v| v.downloaded).count()
    );
    Ok(BookmarkChanges {
        total: all.len(),
        new_count: new_items.len(),
        video_items,
    })
}

/// Stage 2 of the manual sync flow: enqueue the user-confirmed videos.
///
/// Nothing is persisted here — the download history is the cursor, and a video
/// enters it only when its download finishes. Videos the user skips or whose
/// queued task is deleted before completion stay absent from history, so they
/// are offered again on the next sync.
pub async fn confirm_bookmark_enqueue(
    queue: &std::sync::Arc<crate::downloader::queue::DownloadQueue>,
    items: Vec<BookmarkItem>,
) -> Result<ConfirmResult, String> {
    let mut queued = 0usize;
    for item in items.iter().filter(|b| b.has_video) {
        match enqueue_bookmark_item(queue, item) {
            Ok(()) => queued += 1,
            Err(e) => {
                tracing::warn!("bookmarks enqueue failed for {}: {e}", item.tweet_id);
            }
        }
    }
    Ok(ConfirmResult { queued_count: queued })
}

/// Enqueue a single bookmark video into the download queue (same path as a
/// normal "add" deep link). Already-downloaded videos may be re-enqueued — the
/// queue's own duplicate check still rejects links already sitting in the
/// queue, and a re-download simply overwrites the history record (last wins).
fn enqueue_bookmark_item(
    queue: &std::sync::Arc<crate::downloader::queue::DownloadQueue>,
    item: &BookmarkItem,
) -> Result<(), String> {
    let cfg = crate::services::config::ConfigManager::load();
    let output_dir = cfg
        .download_dir
        .clone()
        .unwrap_or_else(|| {
            crate::utils::app_home::AppHome::downloads_dir()
                .to_string_lossy()
                .to_string()
        });
    let base = crate::models::config::DownloadConfig::new(item.url.clone());
    let download_cfg = crate::models::config::DownloadConfig {
        url: item.url.clone(),
        video_id: Some(item.tweet_id.clone()),
        title: Some(item.text.clone()),
        uploader: Some(item.author_name.clone()),
        output_dir: output_dir.clone(),
        format_id: base.format_id,
        output_template: base.output_template,
        ..Default::default()
    };
    // auto_start=false：先排队，等前端两阶段信息获取（fetchVideoInfo →
    // startQueue）后再启动下载，与历史页批量 URL 路径一致；否则任务会
    // 立即开始下载、卡片信息却要等 download-started 兜底才补上。
    // source=BOOKMARK：书签同步入队标记为「书签」来源。
    queue
        .enqueue(
            download_cfg,
            Some(item.text.clone()),
            false,
            None,
            crate::services::download_history::source::BOOKMARK,
        )
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Real-shaped Bookmarks response with two tweets (one video, one text-only)
    /// and a cursor entry at the end.
    fn real_response() -> String {
        r#"{
          "data": {
            "bookmark_timeline_v2": {
              "timeline": {
                "instructions": [
                  {
                    "type": "TimelineAddEntries",
                    "entries": [
                      {
                        "entryId": "tweet-1900000000000000001",
                        "content": {
                          "entryType": "TimelineTimelineItem",
                          "itemContent": {
                            "itemType": "TimelineTweet",
                            "tweet_results": {
                              "result": {
                                "__typename": "Tweet",
                                "rest_id": "1900000000000000001",
                                "core": {
                                  "user_results": {
                                    "result": {
                                      "__typename": "User",
                                      "rest_id": "12345",
                                      "core": {
                                        "screen_name": "prinsss",
                                        "name": "PriN"
                                      }
                                    }
                                  }
                                },
                                "legacy": {
                                  "full_text": "hello world video",
                                  "created_at": "Sat Aug 01 12:00:00 +0000 2026",
                                  "extended_entities": {
                                    "media": [
                                      {
                                        "id_str": "1999999999999999991",
                                        "media_url_https": "https://pbs.twimg.com/media/xyz.jpg",
                                        "type": "video",
                                        "video_info": {
                                          "variants": [
                                            {"bitrate": 1280000, "content_type": "video/mp4", "url": "https://video.twimg.com/.../vid.mp4"}
                                          ]
                                        }
                                      }
                                    ]
                                  }
                                }
                              }
                            }
                          }
                        }
                      },
                      {
                        "entryId": "tweet-1900000000000000002",
                        "content": {
                          "entryType": "TimelineTimelineItem",
                          "itemContent": {
                            "itemType": "TimelineTweet",
                            "tweet_results": {
                              "result": {
                                "__typename": "Tweet",
                                "rest_id": "1900000000000000002",
                                "core": {
                                  "user_results": {
                                    "result": {
                                      "legacy": {
                                        "screen_name": "github",
                                        "name": "GitHub"
                                      }
                                    }
                                  }
                                },
                                "legacy": {
                                  "full_text": "just a text tweet",
                                  "created_at": "Sat Aug 01 11:00:00 +0000 2026"
                                }
                              }
                            }
                          }
                        }
                      },
                      {
                        "entryId": "cursor-bottom-0-1",
                        "content": {
                          "entryType": "TimelineTimelineCursor",
                          "value": "DAABCgABGODx3_IAAA"
                        }
                      }
                    ]
                  }
                ]
              }
            }
          }
        }"#
        .to_string()
    }

    #[test]
    fn build_url_encodes_variables() {
        let url = build_bookmarks_url("AbC123", 100, Some("DAABCgABGODx3_IAAA"));
        assert!(url.starts_with("https://x.com/i/api/graphql/AbC123/Bookmarks?variables="));
        assert!(url.contains("%22count%22%3A100"), "count should be JSON-encoded: {url}");
        assert!(url.contains("DAABCgABGODx3_IAAA"), "cursor should be present: {url}");
    }

    #[test]
    fn build_url_encodes_full_features() {
        let url = build_bookmarks_url("AbC123", 20, None);
        assert!(
            url.contains("%22rweb_video_screen_enabled%22%3Afalse"),
            "real features must be URL-encoded into the query: {url}"
        );
        assert!(
            !url.contains("features=%7B%7D"),
            "features must not be the old empty object: {url}"
        );
    }

    #[test]
    fn build_url_without_cursor() {
        let url = build_bookmarks_url("AbC123", 50, None);
        assert!(url.contains("%22count%22%3A50"));
        assert!(!url.contains("cursor"), "no cursor when None: {url}");
    }

    #[test]
    fn parse_real_response_extracts_tweets() {
        let items = parse_bookmarks_response(&real_response()).expect("parse ok");
        assert_eq!(items.len(), 2);

        let video = &items[0];
        assert_eq!(video.tweet_id, "1900000000000000001");
        assert_eq!(video.handle, "prinsss");
        assert_eq!(video.author_name, "PriN");
        assert_eq!(video.url, "https://x.com/prinsss/status/1900000000000000001");
        assert_eq!(video.text, "hello world video");
        assert!(video.has_video, "media type=video should be detected");

        let text = &items[1];
        assert_eq!(text.handle, "github");
        assert_eq!(text.url, "https://x.com/github/status/1900000000000000002");
        assert!(!text.has_video, "text-only tweet has no video");
    }

    #[test]
    fn extract_cursor_from_real_response() {
        let cursor = extract_cursor(&real_response()).expect("cursor present");
        assert_eq!(cursor, "DAABCgABGODx3_IAAA");
    }

    #[test]
    fn parse_skips_null_result() {
        // Real-world: deleted/restricted tweets come back as result: null.
        let body = r#"{
          "data": {
            "bookmark_timeline_v2": {
              "timeline": {
                "instructions": [
                  {
                    "type": "TimelineAddEntries",
                    "entries": [
                      {
                        "entryId": "tweet-1900000000000000003",
                        "content": {
                          "entryType": "TimelineTimelineItem",
                          "itemContent": {
                            "itemType": "TimelineTweet",
                            "tweet_results": {
                              "result": null
                            }
                          }
                        }
                      }
                    ]
                  }
                ]
              }
            }
          }
        }"#;
        let items = parse_bookmarks_response(body).expect("parse ok");
        assert!(items.is_empty(), "null result must be skipped");
    }

    #[test]
    fn parse_real_response_without_type_field() {
        // The real X response puts `entries` directly on the instruction object
        // WITHOUT a `type: "TimelineAddEntries"` field. Ensure that shape is
        // parsed (older code skipped it because it checked `type` first).
        let body = r#"{
          "data": {
            "bookmark_timeline_v2": {
              "timeline": {
                "instructions": [
                  {
                    "entries": [
                      {
                        "entryId": "tweet-1900000000000000005",
                        "content": {
                          "entryType": "TimelineTimelineItem",
                          "itemContent": {
                            "itemType": "TimelineTweet",
                            "tweet_results": {
                              "result": {
                                "rest_id": "1900000000000000005",
                                "core": {
                                  "user_results": {
                                    "result": {
                                      "legacy": {
                                        "screen_name": "notype",
                                        "name": "No Type"
                                      }
                                    }
                                  }
                                },
                                "legacy": {
                                  "full_text": "instruction without type field"
                                }
                              }
                            }
                          }
                        }
                      }
                    ]
                  }
                ]
              }
            }
          }
        }"#;
        let items = parse_bookmarks_response(body).expect("parse ok");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].handle, "notype");
        assert_eq!(items[0].tweet_id, "1900000000000000005");
    }

    #[test]
    fn parse_animated_gif_counts_as_video() {
        let body = r#"{
          "data": {
            "bookmark_timeline_v2": {
              "timeline": {
                "instructions": [
                  {
                    "type": "TimelineAddEntries",
                    "entries": [
                      {
                        "entryId": "tweet-1900000000000000004",
                        "content": {
                          "entryType": "TimelineTimelineItem",
                          "itemContent": {
                            "itemType": "TimelineTweet",
                            "tweet_results": {
                              "result": {
                                "rest_id": "1900000000000000004",
                                "core": {
                                  "user_results": {
                                    "result": {
                                      "legacy": {
                                        "screen_name": "gifbot",
                                        "name": "Gif Bot"
                                      }
                                    }
                                  }
                                },
                                "legacy": {
                                  "full_text": "an animated gif",
                                  "extended_entities": {
                                    "media": [
                                      {"type": "animated_gif"}
                                    ]
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    ]
                  }
                ]
              }
            }
          }
        }"#;
        let items = parse_bookmarks_response(body).expect("parse ok");
        assert_eq!(items.len(), 1);
        assert!(items[0].has_video, "animated_gif counts as video");
    }

    #[test]
    fn parse_invalid_json_returns_err() {
        let err = parse_bookmarks_response("not json").expect_err("should fail");
        assert!(err.contains("invalid JSON"), "err: {err}");
    }

    #[test]
    fn parse_missing_instructions_returns_err() {
        let err = parse_bookmarks_response(r#"{"data":{}}"#).expect_err("should fail");
        assert!(err.contains("missing"), "err: {err}");
    }

    #[test]
    fn parse_graphql_errors_query_id_expired_hint() {
        // X 对过期 queryId 的典型响应：HTTP 200 + errors 数组。
        let body = r#"{"errors":[{"message":"Could not find query with name 'j5KExFXtSWj8HjRui17ydA'","code":-1,"kind":"other"}]}"#;
        let err = parse_bookmarks_response(body).expect_err("should fail");
        assert!(err.contains("queryId 已失效"), "err: {err}");
        assert!(err.contains("Could not find"), "err: {err}");
    }

    #[test]
    fn parse_graphql_errors_generic_hint() {
        // 非 queryId 相关的 GraphQL 错误走普通前缀。
        let body = r#"{"errors":[{"message":"unauthorized","code":64}]}"#;
        let err = parse_bookmarks_response(body).expect_err("should fail");
        assert!(err.contains("书签接口返回错误"), "err: {err}");
        assert!(err.contains("unauthorized"), "err: {err}");
        assert!(!err.contains("queryId"), "err: {err}");
    }

    #[test]
    fn query_id_hint_detects_not_found_in_non_200_body() {
        // fetch_bookmarks 非 200 分支同样会走 hint（body 片段含特征词）。
        let err = query_id_failure_hint("HTTP 404: {\"errors\":[{\"message\":\"Could not find query\"}]}");
        assert!(err.contains("queryId 已失效"), "err: {err}");
    }

    #[test]
    fn find_new_bookmarks_incremental() {
        let mut previous = HashSet::new();
        previous.insert("1900000000000000001".to_string());

        let items = parse_bookmarks_response(&real_response()).expect("parse ok");
        let fresh = find_new_bookmarks(&previous, &items);

        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].tweet_id, "1900000000000000002");

        // Second pass: nothing new.
        previous.insert("1900000000000000002".to_string());
        let fresh2 = find_new_bookmarks(&previous, &items);
        assert!(fresh2.is_empty());
    }

    #[test]
    fn find_new_bookmarks_empty_previous_returns_all() {
        let items = parse_bookmarks_response(&real_response()).expect("parse ok");
        let fresh = find_new_bookmarks(&HashSet::new(), &items);
        assert_eq!(fresh.len(), 2);
    }

    // ---- Network layer (uses a real local HTTP server, no x.com dependency) ----

    /// Start a tiny local HTTP server that serves the given body for any GET
    /// and captures the request headers it receives. Returns
    /// (addr_string, header_tx, server_handle).
    ///
    /// Must use tokio async IO — `#[tokio::test]` runs a current_thread
    /// runtime, and a blocking `std::net::TcpListener::accept()` would stall
    /// the whole runtime (reqwest never gets scheduled → test hangs).
    fn spawn_mock_server(
        body: String,
        status: u16,
    ) -> (
        String,
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::JoinHandle<()>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // bind() is synchronous (no .await); converting to a tokio listener is
        // also immediate. Only accept/read/write below are async.
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        std_listener
            .set_nonblocking(true)
            .expect("set nonblocking");
        let listener = tokio::net::TcpListener::from_std(std_listener).expect("convert listener");
        let addr = listener
            .local_addr()
            .expect("mock server local addr")
            .to_string();
        let (tx, rx) = tokio::sync::oneshot::channel::<String>();

        let handle = tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                if let Ok(n) = stream.read(&mut buf).await {
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = tx.send(request);
                }
                let reason = if status == 200 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason,
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });

        (addr, rx, handle)
    }

    #[tokio::test]
    async fn fetch_bookmarks_sends_auth_headers_and_parses() {
        let body = real_response();
        let (addr, rx, server) = spawn_mock_server(body, 200);

        let client = reqwest::Client::new();
        let auth = BookmarkAuth {
            auth_token: "test_auth_token".into(),
            ct0: "test_ct0".into(),
        };
        let page = fetch_bookmarks(
            &client,
            &auth,
            "qid123",
            100,
            None,
            &format!("http://{addr}"),
        )
        .await
        .expect("fetch ok");

        assert_eq!(page.status, 200);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].has_video, true);
        assert_eq!(page.next_cursor.as_deref(), Some("DAABCgABGODx3_IAAA"));

        // Verify the request headers actually carried the auth material.
        let request = rx.await.expect("server got request");
        assert!(
            request.to_lowercase().contains("cookie: auth_token=test_auth_token; ct0=test_ct0"),
            "Cookie header missing: {request}"
        );
        assert!(
            request.to_lowercase().contains("x-csrf-token: test_ct0"),
            "x-csrf-token header missing: {request}"
        );
        assert!(
            request.contains("Bearer "),
            "authorization Bearer missing: {request}"
        );
        assert!(
            request.contains("Bookmarks?variables="),
            "GraphQL path missing: {request}"
        );

        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_bookmarks_non_200_returns_err() {
        let (addr, _rx, server) = spawn_mock_server(r#"{"errors":[]}"#.to_string(), 429);

        let client = reqwest::Client::new();
        let auth = BookmarkAuth {
            auth_token: "t".into(),
            ct0: "c".into(),
        };
        let err = fetch_bookmarks(
            &client,
            &auth,
            "qid",
            100,
            None,
            &format!("http://{addr}"),
        )
        .await
        .expect_err("429 should fail");
        assert!(err.contains("429"), "err: {err}");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn fetch_bookmarks_missing_auth_returns_err() {
        let client = reqwest::Client::new();
        let auth = BookmarkAuth::default();
        let err = fetch_bookmarks(&client, &auth, "qid", 100, None, "").await;
        let err = err.expect_err("incomplete auth should fail");
        assert!(err.contains("auth_token"), "err: {err}");
    }

    #[test]
    fn cookie_header_format() {
        let auth = BookmarkAuth {
            auth_token: "abc".into(),
            ct0: "def".into(),
        };
        assert!(auth.is_complete());
        assert_eq!(auth.cookie_header(), "auth_token=abc; ct0=def");
        assert!(!BookmarkAuth::default().is_complete());
    }

    #[test]
    fn parse_x_cookies_extracts_auth_and_ct0() {
        // Real Netscape-format cookie file with both x.com and unrelated entries.
        let path = std::env::temp_dir().join("xdownload_test_cookies.txt");
        let _ = std::fs::remove_file(&path);
        std::fs::write(
            &path,
            concat!(
                "# Netscape HTTP Cookie File\n",
                ".x.com\tTRUE\t/\tTRUE\t0\tauth_token\tSESSIONTOKEN123\n",
                ".x.com\tTRUE\t/\tTRUE\t0\tct0\tCSRFTOKEN456\n",
                ".x.com\tTRUE\t/\tTRUE\t0\tother\tIGNORED\n",
                "example.com\tTRUE\t/\tFALSE\t0\tauth_token\tWRONGDOMAIN\n",
                ".twitter.com\tTRUE\t/\tTRUE\t0\tct0\tTWITTERCT0\n",
            ),
        )
        .expect("write cookie file");

        let auth = parse_x_cookies(&path).expect("parse ok");
        assert_eq!(auth.auth_token, "SESSIONTOKEN123");
        assert_eq!(auth.ct0, "CSRFTOKEN456");
        assert!(auth.is_complete());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn parse_x_cookies_missing_ct0_errors() {
        let path = std::env::temp_dir().join("xdownload_test_cookies_missing.txt");
        let _ = std::fs::remove_file(&path);
        std::fs::write(
            &path,
            ".x.com\tTRUE\t/\tTRUE\t0\tauth_token\tTOKENONLY\n",
        )
        .expect("write cookie file");

        let err = parse_x_cookies(&path).expect_err("missing ct0 should fail");
        assert!(err.contains("ct0"), "err: {err}");

        let _ = std::fs::remove_file(&path);
    }

    // ---- QueryId discovery ----

    #[test]
    fn extract_js_urls_from_html() {
        let html = r#"
            <html>
              <script src="https://abs.twimg.com/foo/main.abc.js"></script>
              <script src="//cdn.example.com/app.js?x=1"></script>
              <script src="/static/home.js"></script>
              <script src="inline.js"></script>
            </html>"#;
        let urls = extract_js_urls(html, "https://x.com");
        // inline.js has no protocol/leading slash — intentionally skipped.
        assert_eq!(urls.len(), 3);
        assert!(urls.contains(&"https://abs.twimg.com/foo/main.abc.js".to_string()));
        assert!(urls.contains(&"https://cdn.example.com/app.js?x=1".to_string()));
        assert!(urls.contains(&"https://x.com/static/home.js".to_string()));
    }

    #[test]
    fn extract_query_id_forward_order() {
        // queryId before operationName (typical X bundle format).
        let js = r#"({queryId:"AbCdEfGh12345678",operationName:"Bookmarks"})"#;
        assert_eq!(
            extract_bookmarks_query_id(js).as_deref(),
            Some("AbCdEfGh12345678")
        );
    }

    #[test]
    fn extract_query_id_reversed_order() {
        let js = r#"{"operationName":"Bookmarks","queryId":"xYz_0123456789ab"}"#;
        assert_eq!(
            extract_bookmarks_query_id(js).as_deref(),
            Some("xYz_0123456789ab")
        );
    }

    #[test]
    fn extract_query_id_mapping_object() {
        let js = r#"operationMapping:{"HomeTimeline":"AAAA","Bookmarks":"QwErTyUiop123456"}"#;
        assert_eq!(
            extract_bookmarks_query_id(js).as_deref(),
            Some("QwErTyUiop123456")
        );
    }

    #[test]
    fn extract_query_id_prefers_bookmarks_not_search() {
        // X has both Bookmarks (list) and BookmarkSearchTimeline (search).
        // We must pick the former — the closed token `"Bookmarks"` must not
        // match `"BookmarkSearchTimeline"`.
        let js = r#"({queryId:"SearchQidValue12345",operationName:"BookmarkSearchTimeline"},{queryId:"BookmarksQid98765",operationName:"Bookmarks"})"#;
        assert_eq!(
            extract_bookmarks_query_id(js).as_deref(),
            Some("BookmarksQid98765")
        );
    }

    #[test]
    fn extract_query_id_skips_search_timeline_only() {
        let js = r#"({queryId:"SearchQidValue12345",operationName:"BookmarkSearchTimeline"})"#;
        assert_eq!(extract_bookmarks_query_id(js), None);
    }

    #[test]
    fn extract_ct0_from_html_finds_token() {
        let html = r#"<script type="application/json">{"ct0":"a1b2C3d4E5f6g7H8","other":1}</script>"#;
        assert_eq!(extract_ct0_from_html(html).as_deref(), Some("a1b2C3d4E5f6g7H8"));
    }

    #[test]
    fn extract_ct0_from_html_missing_returns_none() {
        assert_eq!(extract_ct0_from_html("<html><body></body></html>"), None);
    }

    #[test]
    fn extract_query_id_no_match_returns_none() {
        let js = r#"({queryId:"aaa",operationName:"Bookmarks"})"#; // id too short
        assert_eq!(extract_bookmarks_query_id(js), None);
        let js2 = r#"({queryId:"AbCdEfGh12345678",operationName:"HomeTimeline"})"#;
        assert_eq!(extract_bookmarks_query_id(js2), None);
    }

    #[tokio::test]
    async fn discover_query_id_from_mock_site() {
        // Mock server: serves an HTML page referencing a JS bundle that
        // contains the Bookmarks operation metadata.
        let html_body = r#"
            <html><head>
              <script src="/assets/main.abc123.js"></script>
            </head></html>"#;
        let js_body = r#"
            const ops = {queryId:"QdIs3cr3tValue999",operationName:"Bookmarks"};
        "#;

        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().unwrap();
        let addr_str = addr.to_string();

        let serve = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut served = 0;
            while served < 2 {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                served += 1;
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;
                let request = String::from_utf8_lossy(&buf);
                let body = if request.contains("/assets/main.abc123.js") {
                    js_body
                } else {
                    html_body
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/javascript\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = reqwest::Client::new();
        let auth = BookmarkAuth {
            auth_token: "t".into(),
            ct0: "c".into(),
        };
        let base = format!("http://{addr_str}");
        // Sanity: extract_js_urls on the html_body should yield the bundle URL.
        let urls = extract_js_urls(html_body, &base);
        eprintln!("DEBUG extracted js urls: {urls:?}");
        assert_eq!(urls.len(), 1, "html should expose exactly one bundle url");

        let qid = discover_bookmarks_query_id(&client, &auth, &base, 10)
            .await
            .expect("discover ok");
        assert_eq!(qid, "QdIs3cr3tValue999");

        serve.await.unwrap();
    }

    /// Real-world end-to-end check against x.com. Runs only when explicitly
    /// requested (`--ignored`) because it needs a logged-in browser on this
    /// machine. Browser is hardcoded to `firefox` (this machine's setup).
    /// Writes the first page of bookmarks to
    /// `<workspace>/config/bookmarks_check.json` for inspection.
    #[tokio::test]
    #[ignore = "真实调用 X 需要本机浏览器登录 cookies"]
    async fn real_fetch_from_browser_check() {
        // 用户环境：Firefox 已登录 x.com 并保存了 cookies。
        let browser = "firefox".to_string();
        eprintln!("[bookmarks-check] browser = {browser}");

        // Debug: show masked cookie values to confirm auth_token/ct0 were read.
        if let Ok(auth) = load_bookmark_auth_from_browser(&browser).await {
            let mask = |s: &str| -> String {
                if s.len() <= 6 {
                    s.to_string()
                } else {
                    format!("{}…{}", &s[..3], &s[s.len() - 3..])
                }
            };
            eprintln!(
                "[bookmarks-check] auth_token={} ct0={} complete={}",
                mask(&auth.auth_token),
                mask(&auth.ct0),
                auth.is_complete()
            );

            // Verify the auth_token is actually valid: GET /home without
            // following redirects — 200 = authenticated, 302 = session dead.
            let mut builder = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(15));
            if let Some(proxy) = crate::services::proxy::ProxyConfig::to_reqwest_proxy() {
                builder = builder.proxy(proxy);
            } else if let Ok(p) = reqwest::Proxy::all("http://127.0.0.1:7897") {
                builder = builder.proxy(p);
            }
            let client = builder.build().expect("build probe client");
            let resp = client
                .get("https://x.com/home")
                .header("Cookie", format!("auth_token={};", auth.auth_token))
                .send()
                .await
                .expect("GET /home");
            eprintln!(
                "[bookmarks-check] auth_token probe /home status = {}",
                resp.status().as_u16()
            );
        } else {
            eprintln!("[bookmarks-check] load_bookmark_auth FAILED");
        }

        // Debug: list which x.com cookies were actually exported.
        let cookie_path = std::env::temp_dir().join("xdownload_cookies").join("bookmarks_cookies.txt");
        if let Ok(content) = std::fs::read_to_string(&cookie_path) {
            let names: Vec<&str> = content
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with('#') && l.split('\t').count() >= 7)
                .map(|l| l.split('\t').nth(5).unwrap_or(""))
                .filter(|n| *n != "")
                .collect();
            eprintln!(
                "[bookmarks-check] exported cookie names ({}): {:?}",
                names.len(),
                names
            );
        } else {
            eprintln!("[bookmarks-check] cookie file not found at {cookie_path:?}");
        }

        // 1. Discover the (rotating) queryId from X's web frontend.
        let qid = match discover_bookmarks_query_id_from_browser(&browser).await {
            Ok(q) => {
                eprintln!("[bookmarks-check] discovered queryId = {q}");
                q
            }
            Err(e) => {
                eprintln!(
                    "[bookmarks-check] discover failed ({e}); falling back to default queryId"
                );
                DEFAULT_BOOKMARKS_QUERY_ID.to_string()
            }
        };

        // 2. Fetch the first page of bookmarks with the saved cookies.
        let page = fetch_bookmarks_from_browser(&browser, &qid, 100)
            .await
            .expect("fetch bookmarks failed");
        eprintln!(
            "[bookmarks-check] fetched {} item(s), next_cursor = {:?}, status = {}",
            page.items.len(),
            page.next_cursor,
            page.status
        );
        for (i, item) in page.items.iter().take(20).enumerate() {
            eprintln!(
                "  [{}] id={} video={} handle=@{} text={}",
                i + 1,
                item.tweet_id,
                item.has_video,
                item.handle,
                item.text.chars().take(40).collect::<String>()
            );
        }

        // 3. Persist a JSON snapshot for convenient inspection.
        let snap = serde_json::json!({
            "browser": browser,
            "query_id": qid,
            "count": page.items.len(),
            "next_cursor": page.next_cursor,
            "items": page.items.iter().map(|i| {
                serde_json::json!({
                    "tweet_id": i.tweet_id,
                    "handle": i.handle,
                    "url": i.url,
                    "has_video": i.has_video,
                    "text": i.text,
                    "author_name": i.author_name,
                })
            }).collect::<Vec<_>>(),
        });
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config");
        std::fs::create_dir_all(&out_dir).expect("create config dir");
        let out_path = out_dir.join("bookmarks_check.json");
        std::fs::write(&out_path, serde_json::to_string_pretty(&snap).unwrap())
            .expect("write snapshot");
        eprintln!("[bookmarks-check] snapshot written to {}", out_path.display());

        assert!(
            page.status == 200,
            "unexpected status: {} (auth/rate-limit failure?)",
            page.status
        );
    }

    /// Probe the current x.com web bearer token by scraping the home page.
    /// Prints candidate token strings so we can update `X_WEB_BEARER`.
    #[tokio::test]
    #[ignore = "探查 X 首页 bearer token，需本机网络"]
    async fn probe_web_bearer() {
        let client = build_x_client().expect("client");
        let html = client
            .get("https://x.com/home")
            .send()
            .await
            .expect("GET /home")
            .text()
            .await
            .expect("text");
        eprintln!("[bearer-probe] /home html len = {}", html.len());

        // Look for bearer-like tokens in the raw HTML.
        let re = regex::Regex::new(r#"([A-Za-z0-9%_]{40,})"#).unwrap();
        let mut hits: std::collections::HashSet<String> = std::collections::HashSet::new();
        for m in re.captures_iter(&html) {
            let tok = m[1].to_string();
            if tok.contains("ANRILg") || tok.len() >= 40 {
                hits.insert(tok);
            }
        }
        eprintln!("[bearer-probe] candidate tokens in html: {}", hits.len());
        for t in hits.iter().take(10) {
            eprintln!("  {t}");
        }

        // Also grab JS bundle URLs and scan the first few for a bearer string.
        let urls = extract_js_urls(&html, "https://x.com");
        eprintln!("[bearer-probe] js urls: {}", urls.len());
        for url in urls.iter().take(5) {
            if let Ok(resp) = client.get(url).send().await {
                if let Ok(js) = resp.text().await {
                    let m = regex::Regex::new(r#"(?i)(bearer[^"]{0,80})"#).unwrap();
                    for cap in m.captures_iter(&js).take(5) {
                        eprintln!("  [{}] {}", url, &cap[1].chars().take(60).collect::<String>());
                    }
                }
            }
        }
    }

    #[test]
    fn find_new_bookmarks_returns_only_unseen() {
        let items = vec![
            BookmarkItem {
                tweet_id: "1".into(),
                handle: "a".into(),
                url: "https://x.com/a/status/1".into(),
                text: "t1".into(),
                author_name: "A".into(),
                has_video: true,
            },
            BookmarkItem {
                tweet_id: "2".into(),
                handle: "b".into(),
                url: "https://x.com/b/status/2".into(),
                text: "t2".into(),
                author_name: "B".into(),
                has_video: false,
            },
        ];
        let mut seen = HashSet::new();
        seen.insert("1".to_string());
        let fresh = find_new_bookmarks(&seen, &items);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].tweet_id, "2");
        // Empty previous set → everything is new.
        let all = find_new_bookmarks(&HashSet::new(), &items);
        assert_eq!(all.len(), 2);
        // All seen → nothing new.
        let mut all_seen = HashSet::new();
        all_seen.insert("1".to_string());
        all_seen.insert("2".to_string());
        assert!(find_new_bookmarks(&all_seen, &items).is_empty());
    }

    #[test]
    fn history_id_set_builds_from_download_records() {
        use crate::services::download_history::{DownloadRecord, DownloadStatus};

        fn rec(id: &str, status: DownloadStatus) -> DownloadRecord {
            DownloadRecord {
                video_id: id.to_string(),
                title: None,
                thumbnail: None,
                url: None,
                uploader: None,
                duration: 0,
                view_count: 0,
                like_count: 0,
                file_path: None,
                file_paths: vec![],
                file_size: None,
                downloaded_at: 0,
                status,
                error: None,
                attempts: 0,
                source: crate::services::download_history::source_name(
                    crate::services::download_history::source::SINGLE,
                ),
            }
        }

        // Both successful and failed records count as "processed": the user
        // can retry a failed video from the history page, and it must not be
        // re-offered on every sync.
        let ids = history_id_set(vec![
            rec("A", DownloadStatus::Success),
            rec("B", DownloadStatus::Failed),
        ]);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("A"));
        assert!(ids.contains("B"));

        // find_new_bookmarks excludes everything in the download history.
        let items = vec![
            BookmarkItem {
                tweet_id: "A".into(),
                handle: "a".into(),
                url: "https://x.com/a/status/A".into(),
                text: "t".into(),
                author_name: "A".into(),
                has_video: true,
            },
            BookmarkItem {
                tweet_id: "C".into(),
                handle: "c".into(),
                url: "https://x.com/c/status/C".into(),
                text: "t".into(),
                author_name: "C".into(),
                has_video: true,
            },
        ];
        let fresh = find_new_bookmarks(&ids, &items);
        assert_eq!(fresh.len(), 1); // C is new; A already downloaded
        assert_eq!(fresh[0].tweet_id, "C");
    }

    #[test]
    fn bookmark_video_serializes_flat_with_downloaded_flag() {
        let v = BookmarkVideo {
            item: BookmarkItem {
                tweet_id: "1".into(),
                handle: "a".into(),
                url: "https://x.com/a/status/1".into(),
                text: "t".into(),
                author_name: "A".into(),
                has_video: true,
            },
            downloaded: true,
        };
        let json = serde_json::to_value(&v).expect("serialize ok");
        // flattened: tweet_id sits at the top level, not nested under item.
        assert_eq!(json["tweet_id"], "1");
        assert_eq!(json["has_video"], true);
        assert_eq!(json["downloaded"], true);
    }
}
