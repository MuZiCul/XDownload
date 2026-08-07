use serde::Deserialize;
use tauri::{AppHandle, Emitter};

/// Response from GitHub Releases API
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GithubAsset>,
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

/// Pick the installer download URL from release assets (NSIS .exe preferred,
/// then .msi). Returns None when the release has no installer asset.
fn pick_installer_asset(assets: &[GithubAsset]) -> Option<String> {
    assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(".exe"))
        .or_else(|| assets.iter().find(|a| a.name.to_ascii_lowercase().ends_with(".msi")))
        .map(|a| a.browser_download_url.clone())
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

/// Check for updates by comparing local version against the latest
/// GitHub release. Returns JSON with `has_update`, versions, and release URL.
///
/// References the standard pattern:
///   GET https://api.github.com/repos/{owner}/{repo}/releases/latest
///   → parse tag_name → semver-compare with CARGO_PKG_VERSION
#[tauri::command]
pub async fn check_update() -> serde_json::Value {
    let current = env!("CARGO_PKG_VERSION");

    // Strategy: direct API → proxied API → direct web → proxied web.
    // Direct first: uses the user's own IP, avoiding shared proxy egress IPs
    // that trigger GitHub's unauthenticated rate limit (HTTP 403).
    let direct = match direct_update_client() {
        Ok(c) => c,
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "latest_version": null,
                "current_version": current,
                "url": Option::<String>::None,
                "error": format!("初始化请求失败: {}", e),
            })
        }
    };
    let proxied = update_client().ok();

    // 1. Direct API (best: exposes installer assets, own IP quota).
    if let Some(release) = fetch_latest_release_api(&direct, "MuZiCul", "XDownload").await {
        let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
        let has_update = cmp_semver(latest, current) > 0;
        return serde_json::json!({
            "has_update": has_update,
            "latest_version": latest,
            "current_version": current,
            "url": release.html_url,
            "download_url": pick_installer_asset(&release.assets),
        });
    }

    // 2. Proxied API.
    if let Some(proxy) = &proxied {
        if let Some(release) = fetch_latest_release_api(proxy, "MuZiCul", "XDownload").await {
            let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
            let has_update = cmp_semver(latest, current) > 0;
            return serde_json::json!({
                "has_update": has_update,
                "latest_version": latest,
                "current_version": current,
                "url": release.html_url,
                "download_url": pick_installer_asset(&release.assets),
            });
        }
    }

    // 3. Web fallback (no API rate limit) — direct first, then proxied.
    let mut last_error = String::from("无法检测最新版本");
    let mut web_ok = false;
    let mut web_result = (String::new(), String::new());
    let web_clients = std::iter::once(&direct).chain(proxied.iter());
    for client in web_clients {
        match fetch_latest_tag_via_web(client, "MuZiCul", "XDownload").await {
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
        let has_update = cmp_semver(&latest, current) > 0;
        serde_json::json!({
            "has_update": has_update,
            "latest_version": latest,
            "current_version": current,
            "url": release_url,
            "download_url": Option::<String>::None,
        })
    } else {
        serde_json::json!({
            "has_update": false,
            "latest_version": null,
            "current_version": current,
            "url": Option::<String>::None,
            "error": last_error,
        })
    }
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
/// 2. Fetch latest release version from gyan.dev (our download source)
/// 3. Semver-compare and return result
///
/// Uses the same gyan.dev endpoint that Chocolatey and ffmpeg-sidecar use:
///   GET https://www.gyan.dev/ffmpeg/builds/release-version → "7.1"
#[tauri::command]
pub async fn check_ffmpeg_update() -> serde_json::Value {
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

    // --- Step 3: fetch latest ffmpeg version from gyan.dev ---
    let client = match update_client() {
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

    let resp = match client
        .get("https://www.gyan.dev/ffmpeg/builds/release-version")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "local_version": local_version,
                "latest_version": Option::<String>::None,
                "url": Option::<String>::None,
                "error": format!("网络请求失败: {}", e),
            });
        }
    };

    if !resp.status().is_success() {
        return serde_json::json!({
            "has_update": false,
            "local_version": local_version,
            "latest_version": Option::<String>::None,
            "url": Option::<String>::None,
            "error": format!("gyan.dev 返回 HTTP {}", resp.status().as_u16()),
        });
    }

    let latest = match resp.text().await {
        Ok(t) => t.trim().to_string(),
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "local_version": local_version,
                "latest_version": Option::<String>::None,
                "url": Option::<String>::None,
                "error": format!("读取响应失败: {}", e),
            });
        }
    };

    let has_update = cmp_semver(&latest, &local_version) > 0;

    serde_json::json!({
        "has_update": has_update,
        "local_version": local_version,
        "latest_version": latest,
        "url": "https://www.gyan.dev/ffmpeg/builds/",
    })
}

/// Public re-export for bootstrap command
pub fn parse_ffmpeg_version_export(line: &str) -> Option<String> {
    parse_ffmpeg_version(line)
}

/// Extract "7.1" from "ffmpeg version 7.1-essentials_build-www.gyan.dev Copyright ..."
fn parse_ffmpeg_version(line: &str) -> Option<String> {
    let idx = line.find("ffmpeg version ")?;
    let after = &line[idx + 15..]; // skip "ffmpeg version "
    let version = after.split(['-', ' ', '\t', '\r', '\n']).next()?;
    if version.is_empty() || !version.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some(version.to_string())
}

/// Download the new-version installer into a temp location.
/// Uses the direct-first / proxy-fallback strategy (see bootstrap.rs).
/// Emits "update-download-progress" with `{ percent }`. Returns the local path.
#[tauri::command]
pub async fn download_update(app: AppHandle, url: String) -> Result<String, String> {
    let dest = std::env::temp_dir().join("xdownload-update-setup.exe");
    let _ = std::fs::remove_file(&dest);

    crate::services::bootstrap::Bootstrap::download_with_fallback(
        &url,
        &dest,
        &move |pct| {
            let _ = app.emit(
                "update-download-progress",
                serde_json::json!({ "percent": pct }),
            );
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}

/// Launch the downloaded installer (silent) and exit the current app so the
/// installer can replace the running files. Supports NSIS (.exe) and MSI.
#[tauri::command]
pub fn install_update(app: AppHandle, path: String) -> Result<(), String> {
    let installer = std::path::Path::new(&path);
    if !installer.exists() {
        return Err("更新安装包不存在".to_string());
    }

    let lower = path.to_ascii_lowercase();
    let mut cmd = if lower.ends_with(".msi") {
        let mut c = std::process::Command::new("msiexec");
        c.args(["/i", &path, "/qn"]);
        c
    } else {
        // NSIS installer silent mode
        let mut c = std::process::Command::new(installer);
        c.arg("/S");
        c
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW — avoid a console flash
        cmd.creation_flags(0x08000000);
    }

    cmd.spawn()
        .map_err(|e| format!("启动安装程序失败: {}", e))?;

    tracing::info!("install_update: launched installer {}", installer.display());

    // Clean up child processes (yt-dlp / ffmpeg) so the installer can replace
    // files, then exit — the installer process continues independently.
    crate::utils::process::kill_all_children();
    app.exit(0);
    Ok(())
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
