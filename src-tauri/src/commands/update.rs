use serde::Deserialize;

/// Response from GitHub Releases API
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
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

    let client = match reqwest::Client::builder()
        .user_agent("XDownload")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
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

    let resp = match client
        .get("https://api.github.com/repos/MuZiCul/XDownload/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "latest_version": null,
                "current_version": current,
                "url": Option::<String>::None,
                "error": format!("网络请求失败: {}", e),
            })
        }
    };

    if !resp.status().is_success() {
        return serde_json::json!({
            "has_update": false,
            "latest_version": null,
            "current_version": current,
            "url": Option::<String>::None,
            "error": format!("GitHub API 返回 HTTP {}", resp.status().as_u16()),
        });
    }

    let release: GithubRelease = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "latest_version": null,
                "current_version": current,
                "url": Option::<String>::None,
                "error": format!("解析响应失败: {}", e),
            })
        }
    };

    // Strip optional "v" prefix from tag_name
    let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);

    let has_update = cmp_semver(latest, current) > 0;

    serde_json::json!({
        "has_update": has_update,
        "latest_version": latest,
        "current_version": current,
        "url": release.html_url,
    })
}

/// Check if a newer version of yt-dlp is available.
///
/// 1. Run `yt-dlp --version` to get the local version
/// 2. Fetch latest release from GitHub API
/// 3. Semver-compare and return result
#[tauri::command]
pub async fn check_ytdlp_update() -> serde_json::Value {
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

    let ytdlp_str = ytdlp_path.to_str().unwrap_or("yt-dlp");
    let local_version = match crate::utils::process::execute_with_timeout(
        &[ytdlp_str, "--version"],
        5,
    )
    .await
    {
        Ok(result) if result.is_success() && !result.stdout.is_empty() => {
            result.stdout[0].trim().to_string()
        }
        _ => {
            return serde_json::json!({
                "has_update": false,
                "local_version": Option::<String>::None,
                "latest_version": Option::<String>::None,
                "url": Option::<String>::None,
                "error": "无法获取本地 yt-dlp 版本",
            });
        }
    };

    // --- Step 2: fetch latest yt-dlp release from GitHub ---
    let client = match reqwest::Client::builder()
        .user_agent("XDownload")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
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
        .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
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
            "error": format!("GitHub API 返回 HTTP {}", resp.status().as_u16()),
        });
    }

    let release: GithubRelease = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            return serde_json::json!({
                "has_update": false,
                "local_version": local_version,
                "latest_version": Option::<String>::None,
                "url": Option::<String>::None,
                "error": format!("解析响应失败: {}", e),
            });
        }
    };

    let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);

    let has_update = cmp_semver(latest, &local_version) > 0;

    serde_json::json!({
        "has_update": has_update,
        "local_version": local_version,
        "latest_version": latest,
        "url": release.html_url,
    })
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
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
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
