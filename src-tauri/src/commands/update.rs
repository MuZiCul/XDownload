use serde::Deserialize;

/// Response from GitHub Releases API
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GithubAsset>,
    /// Release 发布时间（ISO 8601 UTC，如 `2026-08-16T09:30:00Z`）。
    /// BtbN master 每次自动构建都会重新发布并刷新该时间，用于时间对比
    /// 检测"是否有新构建"（该场景 tag 恒为 `latest`，无版本号可比）。
    #[serde(default)]
    published_at: Option<String>,
}

/// A release asset (e.g. the NSIS/MSI installer).
#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Build a **direct** (no-proxy) reqwest client for update checks. Tried first:
/// it uses the user's own IP, which avoids the shared proxy egress IP that is
/// much more likely to hit GitHub's unauthenticated API rate limit (HTTP 403).
fn direct_update_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent("XDownload")
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
}

/// Build a reqwest client for update checks that routes through the configured
/// proxy (fallback when the direct connection is blocked).
fn update_client() -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .user_agent("XDownload")
        .timeout(std::time::Duration::from_secs(10));
    if let Some(proxy) = crate::services::proxy::ProxyConfig::to_reqwest_proxy() {
        builder = builder.proxy(proxy);
    }
    builder.build()
}

/// Fetch the latest `GithubRelease` via the GitHub API using `client`.
/// Returns `None` on any request / status / parse failure so callers can fall
/// back to the next strategy (proxy, then the website check).
async fn fetch_latest_release_api(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Option<GithubRelease> {
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        ))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        tracing::warn!(
            "GitHub API {}/{} returned HTTP {}, trying next strategy",
            owner,
            repo,
            resp.status().as_u16()
        );
        return None;
    }
    resp.json::<GithubRelease>().await.ok()
}

/// Fetch the latest release tag by following the `/releases/latest` redirect
/// on the GitHub **website** (not the API, which is rate-limited to ~60
/// unauthenticated requests/hour and can return HTTP 403). Returns the tag
/// (without the "v" prefix) and the resolved release URL.
async fn fetch_latest_tag_via_web(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Result<(String, String), String> {
    let url = format!("https://github.com/{}/{}/releases/latest", owner, repo);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("网页检测失败: {}", e))?;
    let final_url = resp.url().to_string();
    let tag = final_url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if tag.is_empty() {
        return Err(format!("网页检测失败: 无法从 {} 解析版本号", final_url));
    }
    tracing::info!("check via web: {} latest tag = {}", repo, tag);
    Ok((tag, final_url))
}

/// Check if a newer version of yt-dlp is available.
///
/// 1. Run `yt-dlp --version` to get the local version
/// 2. Fetch latest release from GitHub API
/// 3. Semver-compare and return result
#[tauri::command]
pub async fn check_ytdlp_update(local_version: Option<String>) -> serde_json::Value {
    // --- Step 1: get local yt-dlp version ---
    let ytdlp_path = crate::utils::process::find_ytdlp();
    if !ytdlp_path.exists() {
        return serde_json::json!({
            "has_update": false,
            "not_installed": true,
            "local_version": Option::<String>::None,
            "latest_version": Option::<String>::None,
            "url": Option::<String>::None,
        });
    }

    // Reuse the version already fetched by check_ytdlp when the frontend
    // provides it, avoiding a second yt-dlp spawn at startup. Otherwise
    // detect it here with one retry (PyInstaller cold start can be slow).
    let local_version = match local_version {
        Some(v) if !v.is_empty() => v,
        _ => {
            let ytdlp_str = ytdlp_path.to_str().unwrap_or("yt-dlp");
            let mut version: Option<String> = None;
            for attempt in 0..2 {
                let result = crate::utils::process::execute_with_timeout(
                    &[ytdlp_str, "--version"],
                    15,
                )
                .await;
                if let Ok(result) = result {
                    if result.is_success() && !result.stdout.is_empty() {
                        version = Some(result.stdout[0].trim().to_string());
                        break;
                    }
                }
                if attempt == 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
            match version {
                Some(v) => v,
                None => {
                    return serde_json::json!({
                        "has_update": false,
                        "local_version": Option::<String>::None,
                        "latest_version": Option::<String>::None,
                        "url": Option::<String>::None,
                        "error": "无法获取本地 yt-dlp 版本",
                    });
                }
            }
        }
    };

    // --- Step 2: fetch latest yt-dlp release from GitHub ---
    // Strategy: direct API → proxied API → direct web → proxied web.
    let direct = match direct_update_client() {
        Ok(c) => c,
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "local_version": local_version,
                "latest_version": Option::<String>::None,
                "url": Option::<String>::None,
                "error": format!("初始化请求失败: {}", e),
            });
        }
    };
    let proxied = update_client().ok();

    // 1. Direct API (own IP quota — avoids shared-proxy 403 rate limits).
    if let Some(release) = fetch_latest_release_api(&direct, "yt-dlp", "yt-dlp").await {
        let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
        let has_update = cmp_semver(latest, &local_version) > 0;
        return serde_json::json!({
            "has_update": has_update,
            "local_version": local_version,
            "latest_version": latest,
            "url": release.html_url,
        });
    }

    // 2. Proxied API.
    if let Some(proxy) = &proxied {
        if let Some(release) = fetch_latest_release_api(proxy, "yt-dlp", "yt-dlp").await {
            let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
            let has_update = cmp_semver(latest, &local_version) > 0;
            return serde_json::json!({
                "has_update": has_update,
                "local_version": local_version,
                "latest_version": latest,
                "url": release.html_url,
            });
        }
    }

    // 3. Web fallback (no API rate limit) — direct first, then proxied.
    let mut last_error = String::from("无法检测最新版本");
    let mut web_ok = false;
    let mut web_result = (String::new(), String::new());
    let web_clients = std::iter::once(&direct).chain(proxied.iter());
    for client in web_clients {
        match fetch_latest_tag_via_web(client, "yt-dlp", "yt-dlp").await {
            Ok((tag, url)) => {
                web_ok = true;
                web_result = (tag, url);
                break;
            }
            Err(e) => last_error = e,
        }
    }

    if web_ok {
        let (latest, release_url) = web_result;
        let has_update = cmp_semver(&latest, &local_version) > 0;
        serde_json::json!({
            "has_update": has_update,
            "local_version": local_version,
            "latest_version": latest,
            "url": release_url,
        })
    } else {
        serde_json::json!({
            "has_update": false,
            "local_version": local_version,
            "latest_version": Option::<String>::None,
            "url": Option::<String>::None,
            "error": last_error,
        })
    }
}

/// Check if a newer version of ffmpeg is available.
///
/// 1. Run `ffmpeg -version` to get the local version
/// 2. Compare against the latest BtbN master build published on GitHub
///    (download source). Both `N-` (BtbN master) and legacy numeric
///    builds are handled by `check_ffmpeg_master_update`, which compares
///    the remote release time against the local ffmpeg.exe mtime.
///
/// `force_refresh`（前端「检查更新」按钮主动点击时传 true）会绕过远端
/// 发布时间缓存，强制请求 GitHub 并刷新本地缓存。
#[tauri::command]
pub async fn check_ffmpeg_update(force_refresh: Option<bool>) -> serde_json::Value {
    // --- Step 1: check ffmpeg exists ---
    let ffmpeg_path = crate::utils::process::find_ffmpeg();
    if !ffmpeg_path.exists() {
        return serde_json::json!({
            "has_update": false,
            "not_installed": true,
            "local_version": Option::<String>::None,
            "latest_version": Option::<String>::None,
            "url": Option::<String>::None,
        });
    }

    // --- Step 2: get local ffmpeg version ---
    let ffmpeg_str = ffmpeg_path.to_str().unwrap_or("ffmpeg");
    let local_version = match crate::utils::process::execute_with_timeout(
        &[ffmpeg_str, "-version"],
        5,
    )
    .await
    {
        Ok(result) if result.is_success() && !result.stdout.is_empty() => {
            parse_ffmpeg_version(&result.stdout[0])
        }
        _ => {
            return serde_json::json!({
                "has_update": false,
                "local_version": Option::<String>::None,
                "latest_version": Option::<String>::None,
                "url": Option::<String>::None,
                "error": "无法获取本地 ffmpeg 版本",
            });
        }
    };

    let Some(local_version) = local_version else {
        return serde_json::json!({
            "has_update": false,
            "local_version": Option::<String>::None,
            "latest_version": Option::<String>::None,
            "url": Option::<String>::None,
            "error": "无法解析本地 ffmpeg 版本号",
        });
    };

    // 统一走 GitHub（BtbN）检测：基于发布时间 vs 本地 ffmpeg.exe mtime 对比，
    // 不依赖版本号形态（N- 或数字均可），彻底移除 gyan.dev 依赖。
    check_ffmpeg_master_update(&local_version, force_refresh.unwrap_or(false)).await
}

/// ffmpeg 更新检测（唯一入口，适用于所有本地 ffmpeg 构建，含旧版数字
/// 版本）：GitHub 上最新 BtbN master 构建的发布时间（UTC）与本地
/// `ffmpeg.exe` 文件 mtime（解压时已恢复为 zip 内构建打包时刻）对比，
/// 远端发布时间更新 → 有新构建。
///
/// 数据源优先级：API（`published_at`）→ Web（releases/tag/latest 页面的
/// `<relative-time datetime>`）。API 未认证限流 60 次/h，故远端发布时间做
/// **跨进程持久化缓存（24h TTL）**，存 `config/data.db` 的 `config` 表；
/// 24h 内重复检查（含重启后）直接读缓存不再打 API。`force_refresh` 时
/// 绕过缓存强制刷新并回写缓存。全部不可达时静默返回"无更新"。
async fn check_ffmpeg_master_update(local_version: &str, force_refresh: bool) -> serde_json::Value {
    /// 远端发布时间缓存 TTL：固定 24 小时（BtbN master 每天多次构建）。
    const CACHE_TTL_SECS: i64 = 24 * 60 * 60;
    const KEY_PUBLISHED_AT: &str = "ffmpeg_remote_published_at";
    const KEY_FETCHED_AT: &str = "ffmpeg_remote_fetched_at";

    let no_update = || {
        serde_json::json!({
            "has_update": false,
            "local_version": local_version,
            "latest_version": Option::<String>::None,
            "url": "https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest",
        })
    };

    // 跨进程缓存命中（24h 内）→ 复用远端时间，不再请求网络。
    // force_refresh 时忽略缓存直接联网刷新。
    let cached_remote = if !force_refresh {
        load_ffmpeg_remote_cache(KEY_PUBLISHED_AT, KEY_FETCHED_AT, CACHE_TTL_SECS)
    } else {
        tracing::info!("[XDownload] check ffmpeg: force_refresh — bypassing 24h cache");
        None
    };

    // 缓存是否来自网络（非命中）：网络来源才在解析成功后回写缓存。
    let from_network = cached_remote.is_none();

    let remote_at = match cached_remote {
        Some(ts) => {
            tracing::info!(
                "[XDownload] check ffmpeg: 命中 24h 缓存，不联网 (remote_published_at={})",
                ts
            );
            ts
        }
        None => {
            // 数据源 1：GitHub API。
            let mut remote_ts: Option<String> = None;
            if let Ok(client) = update_client() {
                if let Some(release) =
                    fetch_latest_release_api(&client, "BtbN", "FFmpeg-Builds").await
                {
                    if let Some(published) = release.published_at {
                        remote_ts = Some(published);
                    }
                }
            }
            // 数据源 2：Web（API 限流/失败时兜底）——解析 releases 页面 HTML。
            if remote_ts.is_none() {
                remote_ts = fetch_ffmpeg_latest_published_via_web().await;
            }
            let Some(ts) = remote_ts else {
                return no_update();
            };
            ts
        }
    };

    // 解析失败说明远端返回的不是合法时间戳（API/Web 数据异常）——不落盘坏值，
    // 避免污染 24h 缓存；直接返回无更新，下次检查重新请求。
    let Ok(remote) = chrono::DateTime::parse_from_rfc3339(&remote_at) else {
        tracing::warn!(
            "[XDownload] check ffmpeg: 远端发布时间格式异常，不更新缓存: {}",
            remote_at
        );
        return no_update();
    };

    // 仅在拿到合法时间戳后落盘缓存（fetched_at 为当前时刻）。
    // 网络来源（缓存未命中或强制刷新）才写缓存；缓存命中路径不重复写。
    if from_network {
        save_ffmpeg_remote_cache(KEY_PUBLISHED_AT, KEY_FETCHED_AT, &remote_at);
    }
    let date = remote.format("%Y-%m-%d").to_string();
    let remote: std::time::SystemTime = remote.with_timezone(&chrono::Utc).into();

    // 本地基准：ffmpeg.exe 文件 mtime（下载/解压时刻），作为"本地构建时间"。
    let local = crate::utils::process::find_ffmpeg()
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok());
    let has_update = matches!(local, Some(local_time) if remote > local_time);

    // 仅在有更新时返回发布时间日期：前端 `latest !== local` 才渲染琥珀色
    // "最新版本"，无更新时置空以免与绿色"已是最新"状态矛盾。
    let latest_version = has_update.then(|| date);
    serde_json::json!({
        "has_update": has_update,
        "local_version": local_version,
        "latest_version": latest_version,
        "url": "https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest",
    })
}

/// 读取 ffmpeg 远端发布时间跨进程缓存（`config/data.db` 的 `config` 表）。
/// 命中条件：published_at 与 fetched_at 都存在，且 `now - fetched_at < ttl`。
fn load_ffmpeg_remote_cache(
    key_published: &str,
    key_fetched: &str,
    ttl_secs: i64,
) -> Option<String> {
    let conn = crate::services::db::open().ok()?;
    let read = |key: &str| -> Option<String> {
        conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get::<_, String>(0),
        )
        .ok()
    };
    let fetched_at: i64 = read(key_fetched)?.parse().ok()?;
    let published_at = read(key_published)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if now - fetched_at < ttl_secs {
        Some(published_at)
    } else {
        None
    }
}

/// 持久化 ffmpeg 远端发布时间缓存（`config/data.db` 的 `config` 表）。
/// 幂等 upsert；DB 失败时静默忽略（仅失去缓存，下次检查重新请求）。
fn save_ffmpeg_remote_cache(key_published: &str, key_fetched: &str, published_at: &str) {
    let Ok(conn) = crate::services::db::open() else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    for (key, value) in [(key_fetched, now.to_string()), (key_published, published_at.to_string())] {
        let _ = conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        );
    }
}

/// Web fallback：请求 BtbN releases/tag/latest 页面，从 HTML 中提取
/// `<relative-time datetime="...">`（GitHub SSR 自带）作为最新构建发布时间。
/// API 限流/不可达时兜底，同样语义（时间对比），无 API 限流。
async fn fetch_ffmpeg_latest_published_via_web() -> Option<String> {
    let url = "https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest";
    tracing::info!("[XDownload] ffmpeg: API 不可用，尝试 Web fallback 获取发布时间 ({url})");
    let client = match direct_update_client() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("[XDownload] ffmpeg: Web fallback 构建请求客户端失败: {e}");
            return None;
        }
    };
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[XDownload] ffmpeg: Web fallback 请求失败: {e}");
            return None;
        }
    };
    if !response.status().is_success() {
        tracing::warn!(
            "[XDownload] ffmpeg: Web fallback 返回 HTTP {}",
            response.status().as_u16()
        );
        return None;
    }
    let html = match response.text().await {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!("[XDownload] ffmpeg: Web fallback 读取响应失败: {e}");
            return None;
        }
    };
    // 提取第一个 `datetime="..."`（<relative-time datetime="2026-08-16T09:30:00Z">）。
    let marker = "datetime=\"";
    let ts = html
        .find(marker)
        .map(|start| {
            let rest = &html[start + marker.len()..];
            rest[..rest.find('"').unwrap_or(0)].to_string()
        })
        .filter(|s| !s.is_empty());
    match &ts {
        Some(t) => tracing::info!(
            "[XDownload] ffmpeg: Web fallback 获取到发布时间 {}",
            t
        ),
        None => tracing::warn!(
            "[XDownload] ffmpeg: Web fallback 未在页面中找到 <relative-time> 时间戳"
        ),
    }
    ts
}

/// Public re-export for bootstrap command
pub fn parse_ffmpeg_version_export(line: &str) -> Option<String> {
    parse_ffmpeg_version(line)
}

/// Extract a displayable version from an `ffmpeg -version` first line.
///
/// - 常规 release 构建（如 gyan.dev）：`ffmpeg version 7.1-essentials_build-...` → `7.1`
/// - BtbN master 每日构建：`ffmpeg version N-118075-g2424a3f01c-20250101 ...` → `N-118075`
fn parse_ffmpeg_version(line: &str) -> Option<String> {
    let idx = line.find("ffmpeg version ")?;
    let after = &line[idx + 15..]; // skip "ffmpeg version "
    // 先用空白切出第一个 token（`-` 不能用于切分：BtbN master 的
    // `N-118075-g...` 与 release 的 `7.1-essentials-...` 都含 `-`）。
    let token = after.split([' ', '\t', '\r', '\n']).next()?;
    if token.is_empty() {
        return None;
    }
    // 常规 release 构建以数字开头（如 "7.1-essentials_build-..."），取 `-` 前缀。
    if token.starts_with(|c: char| c.is_ascii_digit()) {
        return Some(token.split('-').next()?.to_string());
    }
    // BtbN master 构建：`N-118075-g2424a3f01c-20250101` → `N-118075`。
    if let Some(rest) = token.strip_prefix("N-") {
        let git: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !git.is_empty() {
            return Some(format!("N-{git}"));
        }
    }
    None
}

/// Semver-style comparison: returns positive if a > b, negative if a < b, 0 if equal.
fn cmp_semver(a: &str, b: &str) -> i32 {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().max(vb.len()) {
        let na = va.get(i).copied().unwrap_or(0);
        let nb = vb.get(i).copied().unwrap_or(0);
        match na.cmp(&nb) {
            std::cmp::Ordering::Greater => return 1,
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Equal => {}
        }
    }
    0
}

/// Probe GitHub reachability as a pre-flight check before downloading an app
/// update. Returns the detailed detection result (direct / proxy) so the UI can
/// show the detection outcome and offer proxy configuration when unreachable.
#[tauri::command]
pub async fn check_update_network() -> crate::services::network::GitHubReachability {
    crate::services::network::NetworkDetect::check_github_reachability().await
}

/// Remove tauri-plugin-updater temp files (`%TEMP%\tauri-updater-*`).
///
/// Called when the user aborts an update: the updater caches the downloaded
/// installer under the temp directory, and deleting it stops a pending
/// download/install from being applied (install reads the cached file).
#[tauri::command]
pub fn cleanup_updater_temp() -> Result<(), String> {
    let temp = std::env::temp_dir();
    let mut removed: usize = 0;
    if let Ok(entries) = std::fs::read_dir(&temp) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("tauri-updater-") {
                continue;
            }
            let ok = if path.is_dir() {
                std::fs::remove_dir_all(&path).is_ok()
            } else {
                std::fs::remove_file(&path).is_ok()
            };
            if ok {
                removed += 1;
                tracing::info!("cleanup_updater_temp: removed '{}'", name);
            } else {
                tracing::warn!("cleanup_updater_temp: failed to remove '{}'", name);
            }
        }
    }
    tracing::info!("cleanup_updater_temp: removed {} temp entrie(s)", removed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ffmpeg_version() {
        // 常规 release 构建（gyan.dev）→ 数字版本。
        assert_eq!(
            parse_ffmpeg_version(
                "ffmpeg version 7.1-essentials_build-www.gyan.dev Copyright (c) 2000-2024"
            )
            .as_deref(),
            Some("7.1")
        );
        // BtbN master 每日构建 → N-<git号>。
        assert_eq!(
            parse_ffmpeg_version(
                "ffmpeg version N-118075-g2424a3f01c-20250101 Copyright (c) 2000-2025 the FFmpeg developers"
            )
            .as_deref(),
            Some("N-118075")
        );
        // BtbN release 分支（n8.1）→ 数字版本。
        assert_eq!(
            parse_ffmpeg_version(
                "ffmpeg version 8.1 Copyright (c) 2000-2025 the FFmpeg developers"
            )
            .as_deref(),
            Some("8.1")
        );
        // 无法识别 → None。
        assert_eq!(parse_ffmpeg_version("no version here"), None);
        assert_eq!(
            parse_ffmpeg_version("ffmpeg version N- Copyright (c) 2000-2025").as_deref(),
            None
        );
    }

    #[test]
    fn test_cmp_semver() {
        assert_eq!(cmp_semver("7.1", "7.0"), 1);
        assert_eq!(cmp_semver("8.1", "7.1"), 1);
        assert_eq!(cmp_semver("7.1", "7.1"), 0);
        assert_eq!(cmp_semver("7.0", "7.1"), -1);
        assert_eq!(cmp_semver("N-118075", "8.1"), -1);
    }
}
