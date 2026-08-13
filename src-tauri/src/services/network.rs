use crate::services::proxy::ProxyConfig;
use std::time::Duration;

/// Network environment detection — checks connectivity to key services
/// to determine whether the user is overseas (no proxy needed) or domestic.
pub struct NetworkDetect;

/// Result of a GitHub reachability probe, used as a pre-flight check before
/// downloading app updates. Fields let the UI explain the detection outcome
/// (direct OK → start download; direct failed but proxy OK → download via
/// proxy; both failed → prompt the user to configure a proxy).
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitHubReachability {
    /// Whether `https://github.com` is reachable directly (no proxy).
    pub direct_ok: bool,
    /// Whether a proxy is currently configured/enabled.
    pub proxy_configured: bool,
    /// Whether GitHub is reachable through the configured proxy.
    pub proxy_ok: bool,
    /// Final verdict — reachable via direct or proxy.
    pub reachable: bool,
}

/// Default timeout for quick checks (3 seconds).
const QUICK_TIMEOUT: Duration = Duration::from_secs(3);

/// Default timeout for full checks (5 seconds).
const FULL_TIMEOUT: Duration = Duration::from_secs(5);

impl NetworkDetect {
    /// Build a plain reqwest client (no proxy).
    fn direct_client(timeout: Duration) -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(timeout)
            .build()
            .unwrap_or_default()
    }

    /// Build a client that routes through the configured proxy.
    fn proxy_client(timeout: Duration) -> reqwest::Client {
        let mut builder = reqwest::Client::builder().timeout(timeout);
        if let Some(proxy) = ProxyConfig::to_reqwest_proxy() {
            builder = builder.proxy(proxy);
        }
        builder.build().unwrap_or_default()
    }

    // ==================== Connectivity Checks ====================

    /// Check whether we are overseas (can reach Google without proxy).
    /// Returns true if Google is reachable directly.
    pub async fn is_overseas() -> bool {
        let client = Self::direct_client(QUICK_TIMEOUT);
        match client.head("https://www.google.com").send().await {
            Ok(resp) => resp.status().as_u16() > 0,
            Err(_) => false,
        }
    }

    /// Check whether GitHub is accessible — direct first, then via the
    /// configured proxy. Used to decide if yt-dlp / ffmpeg can be downloaded.
    pub async fn is_github_accessible() -> bool {
        // Fast direct probe first (no proxy).
        {
            let client = Self::direct_client(QUICK_TIMEOUT);
            if let Ok(resp) = client.head("https://github.com").send().await {
                if resp.status().as_u16() > 0 {
                    return true;
                }
            }
        }
        // Fall back to the configured proxy (if any).
        let client = Self::proxy_client(FULL_TIMEOUT);
        match client.head("https://github.com").send().await {
            Ok(resp) => resp.status().as_u16() > 0,
            Err(_) => false,
        }
    }

    /// Check whether Google is accessible (with configured proxy if any).
    /// Used as a pre-flight check before downloading tools.
    pub async fn is_google_accessible() -> bool {
        let client = Self::proxy_client(QUICK_TIMEOUT);
        match client.head("https://www.google.com").send().await {
            Ok(resp) => resp.status().as_u16() > 0,
            Err(_) => false,
        }
    }

    /// Probe GitHub reachability as a pre-flight check before downloading an
    /// app update. Direct probe first (fast, 3s); on failure falls back to the
    /// configured proxy (5s). Returns a structured result so the UI can show
    /// the detection outcome and offer the user a proxy hint when needed.
    pub async fn check_github_reachability() -> GitHubReachability {
        // 1. Direct probe — no proxy.
        {
            let client = Self::direct_client(QUICK_TIMEOUT);
            if let Ok(resp) = client.head("https://github.com").send().await {
                if resp.status().as_u16() > 0 {
                    return GitHubReachability {
                        direct_ok: true,
                        proxy_configured: false,
                        proxy_ok: false,
                        reachable: true,
                    };
                }
            }
        }

        // 2. Fall back to the configured proxy (if any).
        let proxy_configured = ProxyConfig::is_enabled();
        if proxy_configured {
            let client = Self::proxy_client(FULL_TIMEOUT);
            if let Ok(resp) = client.head("https://github.com").send().await {
                if resp.status().as_u16() > 0 {
                    return GitHubReachability {
                        direct_ok: false,
                        proxy_configured: true,
                        proxy_ok: true,
                        reachable: true,
                    };
                }
            }
        }

        GitHubReachability {
            direct_ok: false,
            proxy_configured,
            proxy_ok: false,
            reachable: false,
        }
    }

    /// Check whether x.com is accessible (with configured proxy if any).
    /// Used as a pre-flight check before invoking yt-dlp to avoid long timeouts.
    pub async fn is_x_accessible() -> bool {
        let client = Self::proxy_client(FULL_TIMEOUT);
        match client.head("https://x.com").send().await {
            Ok(resp) => resp.status().as_u16() > 0,
            Err(_) => false,
        }
    }

    // ==================== Proxy Latency Test ====================

    /// Test proxy latency by timing a HEAD request to Google through the given proxy.
    /// Returns Some(latency_ms) on success, None on failure.
    pub async fn test_proxy_latency(host: &str, port: u16) -> Option<i64> {
        let proxy_url = format!("http://{}:{}", host, port);
        let proxy = match reqwest::Proxy::all(&proxy_url) {
            Ok(p) => p,
            Err(_) => return None,
        };

        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(FULL_TIMEOUT)
            .build()
        {
            Ok(c) => c,
            Err(_) => return None,
        };

        let start = std::time::Instant::now();
        match client.head("https://www.google.com").send().await {
            Ok(resp) if resp.status().as_u16() > 0 => {
                Some(start.elapsed().as_millis() as i64)
            }
            _ => None,
        }
    }
}
