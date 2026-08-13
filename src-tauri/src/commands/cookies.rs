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
        return Err("No browser selected".to_string());
    }

    // Check yt-dlp exists before attempting cookie extraction
    let ytdlp_path = crate::utils::process::find_ytdlp();
    if !ytdlp_path.exists() {
        return Err("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp".to_string());
    }

    emit_step(&app, 1, &browser);

    let auth_token = match dump_and_extract_auth_token(&app, &browser).await {
        Ok(t) => {
            emit_step(&app, 2, &browser);
            t
        }
        Err((msg, code)) => {
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
            Ok(serde_json::json!({
                "success": true,
                "message": username,
                "cookie_count": 1,
                "username": username,
                "error_code": null,
            }))
        }
        Err((msg, code)) => {
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
                    Ok(token) => return Ok(token),
                    Err(_) => {
                        // Fall through to error handling
                    }
                }
            }

            // No cookie file or no auth_token found
            let stderr = r.stderr_text();
            let lower = stderr.to_lowercase();
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

    // If x.com redirects us, the cookie is invalid
    if status.is_redirection() {
        return Err((
            "auth_token 已过期或无效，请在浏览器中重新登录 x.com".to_string(),
            "token_invalid",
        ));
    }

    if !status.is_success() {
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
