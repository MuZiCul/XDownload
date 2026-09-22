use crate::services::bootstrap::{KEY_FFMPEG_ASSET_ETAG, KEY_FFMPEG_INSTALLED_AT};
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
    /// 资产最后一次上传时间（ISO 8601 UTC）。BtbN 每次自动构建都会重传同名
    /// 资产（覆盖式），该时间随之变化 —— 用作"远端内容是否变过"的指纹。
    #[serde(default)]
    updated_at: Option<String>,
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

/// 关心的 ffmpeg 资产名（必须与 `bootstrap::FFMPEG_URLS` 的下载目标一致）。
const FFMPEG_ASSET_NAME: &str = "ffmpeg-master-latest-win64-gpl.zip";

/// 没有安装记录时的容差：构建产物打包上传完成后 release 才发布，因此
/// `published_at` 通常比压缩包内文件时间晚几十分钟。用它兜住这段发布延迟，
/// 避免把"刚装完同一份构建"误判成有更新（历史上正是该偏差导致永久误报）。
const RELEASE_LAG_TOLERANCE_SECS: i64 = 6 * 60 * 60;

/// 读取 `config` 表里的一个值。
fn load_config_value(key: &str) -> Option<String> {
    let conn = crate::services::db::open().ok()?;
    conn.query_row(
        "SELECT value FROM config WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

/// 幂等写入 `config` 表（失败静默：只影响更新判定的精度，不影响功能）。
fn save_config_value(key: &str, value: &str) {
    let Ok(conn) = crate::services::db::open() else {
        return;
    };
    let _ = conn.execute(
        "INSERT INTO config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    );
}

/// 更新判定的核心逻辑（纯函数，便于单测）。
///
/// 优先级：
/// 1. **资产 ETag 比对**（最准）：远端资产 ETag ≠ 上次安装的 ETag → 内容变过。
/// 2. **资产上传时刻 vs 本地安装时刻**：两者都是"发布侧"语义的时间，可直接比较；
///    没有安装记录（老版本升级上来）时用 `ffmpeg.exe` mtime + 发布延迟容差兜底。
/// 3. 远端信息拿不到 → 判定无更新（静默，与既有策略一致）。
fn decide_has_update(
    remote_etag: Option<&str>,
    local_etag: Option<&str>,
    remote_asset_at: Option<i64>,
    local_installed_at: Option<i64>,
    local_mtime: Option<i64>,
) -> bool {
    if let (Some(remote), Some(local)) = (remote_etag, local_etag) {
        return remote != local;
    }
    if let Some(remote) = remote_asset_at {
        let base = local_installed_at
            .or_else(|| local_mtime.map(|m| m + RELEASE_LAG_TOLERANCE_SECS));
        return matches!(base, Some(b) if remote > b);
    }
    false
}

/// 探测远端资产的 ETag / Last-Modified：发一次 GET 但**不读 body**（只取响应头，
/// 不会下载整个 zip），直连优先、失败回退代理。任何失败返回 `None`，调用方
/// 自动回退到资产指纹（API）判定。
async fn probe_ffmpeg_asset_fingerprint() -> Option<(String, Option<String>)> {
    let url = format!(
        "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/{}",
        FFMPEG_ASSET_NAME
    );
    let clients = [
        ("direct", direct_update_client().ok()),
        ("proxy", update_client().ok()),
    ];
    for (label, client) in clients.iter() {
        let Some(client) = client else { continue };
        match client.get(&url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    tracing::warn!(
                        "[XDownload] ffmpeg asset fingerprint ({label}): HTTP {}",
                        resp.status().as_u16()
                    );
                    continue;
                }
                let etag = resp
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let last_modified = resp
                    .headers()
                    .get("last-modified")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                // 只读响应头即返回：resp 未读 body，drop 会直接关闭连接。
                drop(resp);
                if let Some(etag) = etag {
                    tracing::info!(
                        "[XDownload] ffmpeg asset fingerprint ({label}): etag={etag}, last_modified={last_modified:?}"
                    );
                    return Some((etag, last_modified));
                }
                tracing::warn!(
                    "[XDownload] ffmpeg asset fingerprint ({label}): 响应头没有 ETag"
                );
            }
            Err(e) => {
                tracing::warn!("[XDownload] ffmpeg asset fingerprint ({label}) 失败: {e}");
            }
        }
    }
    None
}

/// HTTP 日期（`Mon, 21 Sep 2026 14:12:41 GMT`）→ RFC3339（用于缓存）。
fn http_date_to_rfc3339(s: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc2822(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
}

/// RFC3339 时间串 → `YYYY-MM-DD`（前端展示"最新版本日期"用）。
fn rfc3339_to_day(s: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
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

/// ffmpeg 更新检测（唯一入口，适用于所有本地 ffmpeg 构建，含旧版数字版本）。
///
/// 判定优先级：
/// 1. **远端资产 ETag vs 本地记录的 ETag**（最准）：远端只取响应头（不下载
///    body），ETag 不同即说明资产内容变过。
/// 2. **远端资产上传时刻 vs 本地安装时刻**：远端取该资产的 `updated_at`
///    （API `assets[]`，Web 兜底用 release 发布时间），本地取上次成功安装的
///    时刻 —— 两者都是"发布侧"语义，可直接比较。老版本升级上来没有安装记录
///    时，用 `ffmpeg.exe` mtime + [`RELEASE_LAG_TOLERANCE_SECS`] 容差兜底。
///
/// ⚠️ **不要**拿 `release.published_at` 直接和 zip 内文件 mtime 比较：构建产物
/// 打包上传完成后 release 才发布，`published_at` 恒晚于 zip 内文件时间，会导致
/// "装了最新版也永远提示有更新"（v2.9.10 及以前的实际缺陷）。
///
/// 远端信息做**跨进程持久化缓存（24h TTL）**（API 未认证限流 60 次/h），
/// `force_refresh` 时绕过缓存强制刷新并回写。全部不可达时静默返回"无更新"
/// （`up_to_date=false`，与"确认已是最新"区分开）。
async fn check_ffmpeg_master_update(local_version: &str, force_refresh: bool) -> serde_json::Value {
    /// 远端信息缓存 TTL：固定 24 小时（BtbN master 每天多次构建）。
    const CACHE_TTL_SECS: i64 = 24 * 60 * 60;
    /// 远端资产上传时刻（RFC3339）；沿用旧 key，值语义由"release 发布时间"
    /// 升级为"目标资产的上传时刻"。
    const KEY_REMOTE_AT: &str = "ffmpeg_remote_published_at";
    /// 远端目标资产的 ETag。
    const KEY_REMOTE_ETAG: &str = "ffmpeg_remote_asset_etag";
    /// 上次联网成功刷新的时刻（unix 秒）。
    const KEY_FETCHED_AT: &str = "ffmpeg_remote_fetched_at";
    const RELEASE_PAGE_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest";

    // 统一结果：`up_to_date` 明确区分"确认已是最新"与"没检查成功"，
    // 前端据此在下载前做二次校验（避免白下 ~195MB）。
    let result = |has_update: bool, up_to_date: bool, latest: Option<String>| {
        serde_json::json!({
            "has_update": has_update,
            "up_to_date": up_to_date,
            "local_version": local_version,
            "latest_version": latest,
            "url": RELEASE_PAGE_URL,
        })
    };

    // 本地基准：已装内容的 ETag / 安装时刻 / ffmpeg.exe mtime（老版本无前两者）。
    let local_etag = load_config_value(KEY_FFMPEG_ASSET_ETAG).filter(|s| !s.is_empty());
    let local_installed_at =
        load_config_value(KEY_FFMPEG_INSTALLED_AT).and_then(|v| v.parse::<i64>().ok());
    let local_mtime = crate::utils::process::find_ffmpeg()
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    // 远端信息：24h 缓存命中则不联网；否则先探 ETag（只读响应头），
    // 拿不到再走 API/Web 取资产时间。
    let cache_fresh = !force_refresh && ffmpeg_remote_cache_fresh(KEY_FETCHED_AT, CACHE_TTL_SECS);
    let empty = String::new();
    let (remote_at, remote_etag) = if cache_fresh {
        let at = load_config_value(KEY_REMOTE_AT);
        let etag = load_config_value(KEY_REMOTE_ETAG).filter(|s| !s.is_empty());
        tracing::info!(
            "[XDownload] check ffmpeg: 命中 24h 缓存，不联网 (asset_at={at:?}, etag={etag:?})"
        );
        (at, etag)
    } else {
        if force_refresh {
            tracing::info!("[XDownload] check ffmpeg: force_refresh — bypassing 24h cache");
        }
        let probed = probe_ffmpeg_asset_fingerprint().await;
        let mut etag: Option<String> = None;
        let mut at: Option<String> = None;
        if let Some((e, last_modified)) = probed {
            etag = Some(e);
            at = last_modified.as_deref().and_then(http_date_to_rfc3339);
        }
        if at.is_none() {
            at = fetch_ffmpeg_remote_asset_time().await;
        }
        // 两者都拿不到 → 不写缓存（下次检查重新联网）。
        if at.is_some() || etag.is_some() {
            save_ffmpeg_remote_cache(KEY_REMOTE_AT, KEY_FETCHED_AT, at.as_deref());
            save_config_value(KEY_REMOTE_ETAG, etag.as_deref().unwrap_or(&empty));
        }
        (at, etag)
    };

    // 1) ETag 比对（双方都有才有意义）。
    if let (Some(remote), Some(local)) = (remote_etag.as_deref(), local_etag.as_deref()) {
        let has_update = decide_has_update(Some(remote), Some(local), None, None, None);
        tracing::info!(
            "[XDownload] check ffmpeg: ETag 比对 local={local} remote={remote} -> has_update={has_update}"
        );
        let latest = has_update
            .then(|| remote_at.as_deref().and_then(rfc3339_to_day))
            .flatten();
        return result(has_update, !has_update, latest);
    }

    // 2) 资产上传时刻 vs 本地安装时刻（老版本无安装记录 → mtime + 容差）。
    let remote_ts = remote_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp());
    let has_update = decide_has_update(
        None,
        None,
        remote_ts,
        local_installed_at,
        local_mtime,
    );
    tracing::info!(
        "[XDownload] check ffmpeg: asset_at={remote_at:?}, installed_at={local_installed_at:?}, mtime={local_mtime:?} -> has_update={has_update}"
    );

    // 仅在有更新时返回日期：前端 `latest !== local` 才渲染琥珀色"最新版本"，
    // 无更新时置空以免与绿色"已是最新"状态矛盾。
    let latest = has_update
        .then(|| remote_at.as_deref().and_then(rfc3339_to_day))
        .flatten();
    result(has_update, remote_ts.is_some() && !has_update, latest)
}

/// 取远端资产的"内容时间"：优先目标资产的 `updated_at`（API `assets[]`），
/// 退回 release `published_at`，再退回 releases 页面的 `<relative-time>`。
async fn fetch_ffmpeg_remote_asset_time() -> Option<String> {
    if let Ok(client) = update_client() {
        if let Some(release) = fetch_latest_release_api(&client, "BtbN", "FFmpeg-Builds").await {
            if let Some(ts) = release
                .assets
                .iter()
                .find(|a| a.name == FFMPEG_ASSET_NAME)
                .and_then(|a| a.updated_at.clone())
            {
                tracing::info!("[XDownload] ffmpeg remote asset updated_at = {ts}");
                return Some(ts);
            }
            if let Some(published) = release.published_at {
                tracing::info!(
                    "[XDownload] ffmpeg: 未找到资产 {}，退回 release published_at = {published}",
                    FFMPEG_ASSET_NAME
                );
                return Some(published);
            }
        }
    }
    fetch_ffmpeg_latest_published_via_web().await
}

/// 远端信息缓存是否仍在 TTL 内（`now - fetched_at < ttl`）。
fn ffmpeg_remote_cache_fresh(key_fetched: &str, ttl_secs: i64) -> bool {
    let Some(fetched_at) = load_config_value(key_fetched).and_then(|v| v.parse::<i64>().ok())
    else {
        return false;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    now - fetched_at < ttl_secs
}

/// 持久化 ffmpeg 远端信息缓存（`config/data.db` 的 `config` 表）：
/// 资产上传时刻（可能拿不到，写空串）+ 本次联网刷新的时刻（决定 24h TTL）。
fn save_ffmpeg_remote_cache(key_at: &str, key_fetched: &str, at: Option<&str>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    save_config_value(key_fetched, &now.to_string());
    save_config_value(key_at, at.unwrap_or(""));
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

    // ---- ffmpeg 更新判定（v2.9.11：修复"装了最新版仍提示更新"）----

    /// 本次修复的真实案例：资产上传于 14:12，zip 内文件构建于 13:23（早 49 分钟），
    /// 用户在发布之后才安装 → 不该提示更新。
    /// （旧逻辑拿 release `published_at` 直接与 mtime 比，恒判"有更新"。）
    #[test]
    fn decide_no_update_when_installed_after_release() {
        let asset_at = 1_789_999_961; // 2026-09-21T14:12:41Z
        let stale_mtime = asset_at - 2_947; // 13:23:34Z（差 49 分 7 秒，zip 内构建时间）
        let installed_at = asset_at + 15_000; // 发布之后才安装
        assert!(!decide_has_update(
            None,
            None,
            Some(asset_at),
            Some(installed_at),
            Some(stale_mtime)
        ));
    }

    /// 老版本升级上来（无安装记录）：mtime 只比资产时间早几十分钟（发布延迟），
    /// 落在容差内 → 判定"无更新"，存量用户升级后立刻不再误报。
    #[test]
    fn decide_no_update_within_release_lag_tolerance() {
        let asset_at = 1_789_999_961;
        let mtime = asset_at - 2_947;
        assert!(!decide_has_update(
            None,
            None,
            Some(asset_at),
            None,
            Some(mtime)
        ));
    }

    /// 无安装记录且本地确实很旧（超出容差）→ 提示有更新。
    #[test]
    fn decide_update_when_local_is_old() {
        let asset_at = 1_789_999_961;
        let mtime = asset_at - RELEASE_LAG_TOLERANCE_SECS - 60;
        assert!(decide_has_update(
            None,
            None,
            Some(asset_at),
            None,
            Some(mtime)
        ));
    }

    /// ETag 一致 → 内容没变；不一致 → 有更新（最精确的路径）。
    #[test]
    fn decide_by_etag() {
        assert!(!decide_has_update(
            Some("W/\"abc\""),
            Some("W/\"abc\""),
            None,
            None,
            None
        ));
        assert!(decide_has_update(
            Some("W/\"def\""),
            Some("W/\"abc\""),
            None,
            None,
            None
        ));
    }

    /// 远端信息拿不到 → 静默"无更新"（不误报）；本地有 ETag 但远端探测失败
    /// 时退回时间判定，两者都无 → false。
    #[test]
    fn decide_no_update_without_remote_info() {
        assert!(!decide_has_update(None, None, None, Some(1), Some(2)));
        assert!(!decide_has_update(None, Some("W/\"abc\""), None, None, None));
    }

    #[test]
    fn test_http_date_helpers() {
        assert_eq!(
            http_date_to_rfc3339("Mon, 21 Sep 2026 14:12:41 GMT").as_deref(),
            Some("2026-09-21T14:12:41+00:00")
        );
        assert_eq!(
            rfc3339_to_day("2026-09-21T14:12:41+00:00").as_deref(),
            Some("2026-09-21")
        );
        assert_eq!(http_date_to_rfc3339("not a date"), None);
        assert_eq!(rfc3339_to_day("not a date"), None);
    }

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
