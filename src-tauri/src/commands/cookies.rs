use crate::services::cookies::CookieManager;
use crate::services::proxy::ProxyConfig;
use std::io::BufRead;
use tauri::{AppHandle, Emitter};

/// Validate cookies from a browser by:
/// 1. Dumping cookies from the browser to a temp file via yt-dlp
/// 2. Parsing the Netscape cookie file to find x.com's `auth_token`
/// 3. Calling x.com API with the auth_token to verify login and extract username
///
/// Progress messages are emitted via `cookies-progress` event so the frontend
/// can display step-by-step status.
#[tauri::command]
pub async fn validate_cookies(app: AppHandle, browser: String) -> Result<serde_json::Value, String> {
    if browser.is_empty() || browser == "none" {
        tracing::warn!("[XDownload] validate_cookies: no browser selected");
        return Err("No browser selected".to_string());
    }
    tracing::info!("[XDownload] validate_cookies start: browser={}", browser);

    // Check yt-dlp exists before attempting cookie extraction
    let ytdlp_path = crate::utils::process::find_ytdlp();
    if !ytdlp_path.exists() {
        tracing::warn!("[XDownload] validate_cookies: yt-dlp not installed");
        return Err("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp".to_string());
    }

    emit_step(&app, 1, &browser);

    let auth_token = match dump_and_extract_auth_token(&app, &browser).await {
        Ok(t) => {
            emit_step(&app, 2, &browser);
            t
        }
        Err((msg, code)) => {
            tracing::warn!(
                "[XDownload] validate_cookies: cookie extraction failed browser={} error_code={} msg={}",
                browser,
                code,
                msg
            );
            emit_fail(&app, &browser, code);
            return Ok(serde_json::json!({
                "success": false,
                "message": msg,
                "cookie_count": 0,
                "error_code": code,
            }));
        }
    };

    // Verify the token works on x.com and get the username
    match verify_x_auth_token(&auth_token).await {
        Ok(username) => {
            emit_step(&app, 3, &browser);
            tracing::info!(
                "[XDownload] validate_cookies success: browser={} username={}",
                browser,
                username
            );
            Ok(serde_json::json!({
                "success": true,
                "message": username,
                "cookie_count": 1,
                "username": username,
                "error_code": null,
            }))
        }
        Err((msg, code)) => {
            tracing::warn!(
                "[XDownload] validate_cookies: token verify failed browser={} error_code={} msg={}",
                browser,
                code,
                msg
            );
            emit_fail(&app, &browser, code);
            Ok(serde_json::json!({
                "success": false,
                "message": msg,
                "cookie_count": 0,
                "error_code": code,
            }))
        }
    }
}

/// Get the current x.com username associated with the saved cookie source.
///
/// Unlike `validate_cookies`, this is a **silent** lookup used by the UI (e.g.
/// the bookmarks page header) to display "who is logged in". It:
/// - Returns `null` when no cookie source is configured, instead of an error.
/// - Does NOT emit `cookies-progress` events (no spinner expected).
/// - Logs failures (missing yt-dlp / no auth_token / network) for debugging.
///
/// 固化缓存（避免每次重复 dump ~17s）：
/// - 成功验证后把「browser + sha256(auth_token) 指纹 + 用户名」固化到
///   `config/data.db` 的 `config` 表。
/// - `force=false`（默认，挂载/自动触发）：**先查缓存**——若固化的 browser
///   与当前一致 → 直接返回缓存用户名，**跳过 yt-dlp dump（0 网络，秒回）**。
/// - `force=true`（用户手动点「获取用户」）：跳过缓存，强制 dump + verify，
///   感知同浏览器内 cookies 内容变化（重新登录等）。
/// - 未配置 cookies 来源 → 返回 `null`（不触发任何探测）。
#[tauri::command]
pub async fn get_cookies_username(app: AppHandle, force: Option<bool>) -> Option<String> {
    use sha2::{Digest, Sha256};

    let browser = crate::services::config::ConfigManager::load_cookie_source()?;
    tracing::info!(
        "[XDownload] get_cookies_username: browser={} force={}",
        browser,
        force.unwrap_or(false)
    );

    // 非强制（自动触发）且 browser 未变 → 直接用固化用户名，跳过 dump（0 网络）。
    if !force.unwrap_or(false) {
        if let Some(cached) = load_cookies_username_cache_by_browser(&browser) {
            tracing::info!(
                "[XDownload] get_cookies_username: cache hit by browser (no dump)"
            );
            return Some(cached);
        }
    }

    // yt-dlp is required to dump browser cookies.
    let ytdlp_path = crate::utils::process::find_ytdlp();
    if !ytdlp_path.exists() {
        tracing::warn!("[XDownload] get_cookies_username: yt-dlp not installed");
        return None;
    }

    let auth_token = match dump_and_extract_auth_token(&app, &browser).await {
        Ok(t) => t,
        Err((msg, code)) => {
            tracing::warn!(
                "[XDownload] get_cookies_username: cookie extraction failed browser={} error_code={} msg={}",
                browser,
                code,
                msg
            );
            return None;
        }
    };

    // 当前 cookies 指纹（auth_token 的 SHA-256）。
    let fingerprint = {
        let mut hasher = Sha256::new();
        hasher.update(auth_token.as_bytes());
        hasher.finalize()
    };
    let fingerprint_hex = format!("{:x}", fingerprint);

    // verify 可能因网络/代理偶发失败：每 5 秒重试一次，最多 3 次。
    // 全部失败才返回 null（静默）；任何一次成功即固化并返回。
    const MAX_ATTEMPTS: usize = 3;
    const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

    for attempt in 1..=MAX_ATTEMPTS {
        match verify_x_auth_token(&auth_token).await {
            Ok(username) => {
                tracing::info!(
                    "[XDownload] get_cookies_username: success browser={} username={} (attempt={})",
                    browser,
                    username,
                    attempt
                );
                // 固化「browser + 指纹 ↔ 用户名」关联。
                save_cookies_username_cache(&browser, &fingerprint_hex, &username);
                return Some(username);
            }
            Err((msg, code)) => {
                tracing::warn!(
                    "[XDownload] get_cookies_username: token verify failed browser={} error_code={} msg={} (attempt={}/{})",
                    browser,
                    code,
                    msg,
                    attempt,
                    MAX_ATTEMPTS
                );
                if attempt < MAX_ATTEMPTS {
                    tokio::time::sleep(RETRY_INTERVAL).await;
                }
            }
        }
    }
    tracing::warn!(
        "[XDownload] get_cookies_username: giving up after {} attempts browser={}",
        MAX_ATTEMPTS,
        browser
    );
    // verify 全部失败 → 固化「browser ↔ @None」占位（不清空），
    // 表示当前 cookies 无有效用户。前端把 @None 视为未登录（禁用同步/查看）。
    // 下次 force=false 命中缓存直接返回 @None，避免反复 dump。
    save_cookies_username_cache(&browser, &fingerprint_hex, "@None");
    None
}

/// 按 browser 读取固化的「cookies ↔ 用户名」缓存。
/// 仅当固化的 browser 与当前一致时返回用户名；browser 不同/无缓存返回 None。
/// 用于非强制（自动触发）路径：browser 未变 → 跳过 yt-dlp dump，秒回。
fn load_cookies_username_cache_by_browser(browser: &str) -> Option<String> {
    let conn = crate::services::db::open().ok()?;
    let cached_browser: Option<String> = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'cookies_username_browser'",
            [],
            |row| row.get(0),
        )
        .ok();
    if cached_browser.as_deref() != Some(browser) {
        return None;
    }
    conn.query_row(
        "SELECT value FROM config WHERE key = 'cookies_username'",
        [],
        |row| row.get(0),
    )
    .ok()
}

/// 固化「browser + 指纹 ↔ 用户名」到 config/data.db（幂等 upsert）。
fn save_cookies_username_cache(browser: &str, fingerprint: &str, username: &str) {
    let Ok(conn) = crate::services::db::open() else {
        return;
    };
    for (key, value) in [
        ("cookies_username_browser", browser.to_string()),
        ("cookies_username_fingerprint", fingerprint.to_string()),
        ("cookies_username", username.to_string()),
    ] {
        let _ = conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        );
    }
    tracing::info!(
        "[XDownload] cookies username cache persisted: browser={} fingerprint={} username={}",
        browser,
        &fingerprint[..fingerprint.len().min(12)],
        username
    );
}

/// Emit a structured progress step (1 = extracting, 2 = verifying, 3 = success).
fn emit_step(app: &AppHandle, step: u8, browser: &str) {
    let _ = app.emit(
        "cookies-progress",
        serde_json::json!({ "step": step, "browser": browser }),
    );
}

/// Emit a structured failure step (0 = failed, with an error_code for i18n).
fn emit_fail(app: &AppHandle, browser: &str, code: &str) {
    let _ = app.emit(
        "cookies-progress",
        serde_json::json!({
            "step": 0,
            "browser": browser,
            "error_code": code,
        }),
    );
}

/// Dump browser cookies to a temp file via yt-dlp and extract x.com `auth_token`.
/// Returns `(message, error_code)` on failure.
async fn dump_and_extract_auth_token(
    _app: &AppHandle,
    browser: &str,
) -> Result<String, (String, &'static str)> {
    let temp_dir = std::env::temp_dir().join("xdownload_cookies");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| (format!("创建临时目录失败: {}", e), "unknown"))?;

    let cookie_file = temp_dir.join("cookies.txt");
    let _ = std::fs::remove_file(&cookie_file);

    let ytdlp = crate::utils::process::find_ytdlp();
    let ytdlp_str = ytdlp.to_str().unwrap_or("yt-dlp").to_string();

    // Use --print-to-file to dump cookies without requiring a valid URL
    let mut args: Vec<String> = vec![
        ytdlp_str,
        "--cookies-from-browser".to_string(),
        browser.to_string(),
        "--cookies".to_string(),
        cookie_file.to_string_lossy().to_string(),
        "--no-warnings".to_string(),
        "--no-color".to_string(),
    ];

    // Add proxy if configured
    if let Some(proxy_url) = ProxyConfig::to_proxy_url() {
        args.push("--proxy".to_string());
        args.push(proxy_url);
    }

    // Use --dump-user-agent trick: give a valid URL that yt-dlp can parse
    // but skip download (we only need cookies dumped)
    args.push("--skip-download".to_string());
    args.push("--flat-playlist".to_string());
    args.push("https://x.com/home".to_string());

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let result = crate::utils::process::execute_with_timeout(&args_refs, 30).await;

    match result {
        Ok(r) => {
            // Even if yt-dlp reports failure (e.g. unsupported URL),
            // cookies may still have been written to the file — check it.
            if cookie_file.exists() {
                match parse_auth_token(&cookie_file) {
                    Ok(token) => {
                        tracing::info!(
                            "[XDownload] dump cookies: browser={} auth_token extracted",
                            browser
                        );
                        return Ok(token);
                    }
                    Err(_) => {
                        // Fall through to error handling
                    }
                }
            }

            // No cookie file or no auth_token found
            let stderr = r.stderr_text();
            let lower = stderr.to_lowercase();
            tracing::warn!(
                "[XDownload] dump cookies: no auth_token found browser={} stderr={}",
                browser,
                stderr.lines().last().unwrap_or("").trim()
            );
            if lower.contains("could not copy")
                || (lower.contains("copy") && lower.contains("database"))
            {
                Err((
                    format!("{} 正在运行，Cookie 数据库被锁定，请关闭浏览器后重试", browser),
                    "browser_locked",
                ))
            } else if lower.contains("could not find") || lower.contains("not found") {
                Err((
                    format!("未找到 {} Cookie 数据库（浏览器未安装或从未使用）", browser),
                    "browser_not_found",
                ))
            } else {
                Err((
                    format!(
                        "未在浏览器 cookie 中找到 x.com 的 auth_token，请确保已在浏览器中登录 x.com"
                    ),
                    "no_auth_token",
                ))
            }
        }
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!(
                "[XDownload] dump cookies: yt-dlp failed browser={} error={}",
                browser,
                msg
            );
            if msg.contains("timeout") {
                Err((format!("{} 验证超时", browser), "timeout"))
            } else {
                Err((format!("cookies 提取异常: {}", msg), "unknown"))
            }
        }
    }
}

/// Parse a Netscape-format cookie file for x.com's `auth_token`.
fn parse_auth_token(path: &std::path::Path) -> Result<String, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("无法读取 cookie 文件: {}", e))?;
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line.unwrap_or_default();
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 {
            let domain = parts[0];
            let name = parts[5];
            let value = parts[6];
            if (domain == ".x.com" || domain == "x.com") && name == "auth_token" && !value.is_empty() {
                return Ok(value.to_string());
            }
        }
    }

    Err("未找到 auth_token".to_string())
}

/// Verify the auth_token by fetching x.com/home with the cookie.
/// If the cookie is valid, the page returns a 200 with the user's info embedded.
/// If invalid or expired, x.com redirects to the login page (302 or different content).
/// Returns `(message, error_code)` on failure.
async fn verify_x_auth_token(auth_token: &str) -> Result<String, (String, &'static str)> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none()) // don't follow redirects — 302 = not authenticated
        .timeout(std::time::Duration::from_secs(15));

    if let Some(proxy) = ProxyConfig::to_reqwest_proxy() {
        builder = builder.proxy(proxy);
    }

    let client = builder
        .build()
        .map_err(|e| (format!("创建请求客户端失败: {}", e), "unknown"))?;

    let resp = client
        .get("https://x.com/home")
        .header("Cookie", format!("auth_token={};", auth_token))
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ("请求超时，请检查网络或代理".to_string(), "timeout")
            } else {
                (format!("无法访问 x.com: {}", e), "network")
            }
        })?;

    let status = resp.status();
    tracing::info!(
        "[XDownload] verify auth_token: x.com/home status={}",
        status.as_u16()
    );

    // If x.com redirects us, the cookie is invalid
    if status.is_redirection() {
        tracing::warn!(
            "[XDownload] verify auth_token: redirect ({}), token invalid/expired",
            status.as_u16()
        );
        return Err((
            "auth_token 已过期或无效，请在浏览器中重新登录 x.com".to_string(),
            "token_invalid",
        ));
    }

    if !status.is_success() {
        tracing::warn!(
            "[XDownload] verify auth_token: x.com error status={}",
            status.as_u16()
        );
        return Err((
            format!("x.com 返回错误 (HTTP {})", status.as_u16()),
            "token_invalid",
        ));
    }

    let body = resp.text().await.unwrap_or_default();

    // Search for "screen_name":"..." in the embedded JSON data
    // x.com home page includes user data in a __NEXT_DATA__ or similar JSON blob
    if let Some(screen_name) = extract_screen_name_from_html(&body) {
        return Ok(screen_name);
    }

    // Fallback: look for "screen_name":"..." anywhere
    if let Some(idx) = body.find("screen_name") {
        let after = &body[idx..];
        if let Some(colon) = after.find(':') {
            let value_part = after[colon + 1..].trim();
            if let Some(start) = value_part.find('"') {
                let inner = &value_part[start + 1..];
                if let Some(end) = inner.find('"') {
                    return Ok(inner[..end].to_string());
                }
            }
        }
    }

    Err((
        "未能从 x.com 页面中解析出用户名，可能页面结构已变更".to_string(),
        "parse",
    ))
}

/// Extract screen_name from x.com HTML by looking for it in embedded JSON state.
fn extract_screen_name_from_html(html: &str) -> Option<String> {
    // Pattern: "screen_name":"username"
    // Look for it in the <script> JSON data sections
    let mut search = html;
    while let Some(pos) = search.find("\"screen_name\"") {
        let after = &search[pos + 14..]; // skip past "screen_name"
        let after = after.trim_start();
        if after.starts_with(':') {
            let value_part = after[1..].trim_start();
            if value_part.starts_with('"') {
                let inner = &value_part[1..];
                if let Some(end) = inner.find('"') {
                    let name = &inner[..end];
                    // Validate it looks like a real username (not empty, reasonable length)
                    if !name.is_empty() && name.len() <= 30 && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Some(name.to_string());
                    }
                }
            }
        }
        search = &search[pos + 14..];
    }
    None
}

/// Scan for available browser cookies and return the first one found
#[tauri::command]
pub fn scan_cookies() -> Option<String> {
    CookieManager::scan_available_browser()
}

/// Return the list of browsers installed on this machine (registry /
/// executable presence). The frontend uses this to only offer installed
/// browsers in the cookie source dropdown.
#[tauri::command]
pub fn list_browsers() -> Vec<String> {
    CookieManager::installed_browsers()
}
