use std::sync::{Mutex, OnceLock};

// ==================== Global Proxy State ====================

/// Thread-safe global proxy state.
static PROXY_STATE: OnceLock<Mutex<ProxyState>> = OnceLock::new();

fn state() -> &'static Mutex<ProxyState> {
    PROXY_STATE.get_or_init(|| Mutex::new(ProxyState::default()))
}

/// Internal mutable proxy state.
#[derive(Debug, Clone)]
struct ProxyState {
    /// Proxy URL scheme, e.g. "http", "socks5", "https".
    scheme: String,
    host: String,
    port: u16,
    enabled: bool,
    from_system_proxy: bool,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            scheme: "http".to_string(),
            host: String::new(),
            port: 0,
            enabled: false,
            from_system_proxy: false,
        }
    }
}

// ==================== ProxyTestResult ====================

/// Result of a proxy connectivity test.
#[derive(Debug, Clone)]
pub struct ProxyTestResult {
    pub success: bool,
    pub http_status: i32,
    pub elapsed_ms: u64,
    pub message: String,
}

impl ProxyTestResult {
    pub fn new(success: bool, http_status: i32, elapsed_ms: u64, message: String) -> Self {
        Self {
            success,
            http_status,
            elapsed_ms,
            message,
        }
    }
}

// ==================== Public API ====================

pub struct ProxyConfig;

impl ProxyConfig {
    /// Manual proxy override (HTTP scheme).
    pub fn set_proxy(host: &str, port: u16) {
        Self::set_proxy_full(host, port, "http");
    }

    /// Manual proxy override with an explicit scheme ("http", "socks5", ...).
    pub fn set_proxy_full(host: &str, port: u16, scheme: &str) {
        let mut s = state().lock().unwrap();
        s.host = host.to_string();
        s.port = port;
        s.scheme = if scheme.is_empty() {
            "http".to_string()
        } else {
            scheme.to_string()
        };
        s.enabled = true;
        s.from_system_proxy = false;
    }

    /// Get the configured proxy scheme ("http", "socks5", ...).
    pub fn get_proxy_scheme() -> String {
        state().lock().unwrap().scheme.clone()
    }

    /// Disable proxy (but keep settings).
    pub fn disable() {
        state().lock().unwrap().enabled = false;
    }

    /// Re-enable the existing proxy without altering host/port/scheme.
    /// Used to restore a previously disabled proxy (e.g. after disabling then
    /// re-enabling system proxy), preserving the from_system_proxy marker.
    pub fn enable() {
        let mut s = state().lock().unwrap();
        s.enabled = true;
    }

    /// Whether a proxy is active.
    pub fn is_enabled() -> bool {
        let s = state().lock().unwrap();
        s.enabled && !s.host.is_empty()
    }

    /// Get the configured proxy host.
    pub fn get_proxy_host() -> String {
        state().lock().unwrap().host.clone()
    }

    /// Get the configured proxy port.
    pub fn get_proxy_port() -> u16 {
        state().lock().unwrap().port
    }

    /// Whether the current proxy was detected from the Windows system registry.
    pub fn is_from_system_proxy() -> bool {
        state().lock().unwrap().from_system_proxy
    }

    /// Returns "scheme://host:port" format for display.
    pub fn get_proxy_string() -> String {
        let s = state().lock().unwrap();
        if !s.enabled || s.host.is_empty() {
            return "none".to_string();
        }
        format!("{}://{}:{}", s.scheme, s.host, s.port)
    }

    /// Returns "--proxy scheme://host:port" for yt-dlp CLI (kept for
    /// compatibility; prefer [`Self::to_proxy_url`] for direct use).
    pub fn to_cli_args() -> String {
        let s = state().lock().unwrap();
        if !s.enabled || s.host.is_empty() {
            return String::new();
        }
        format!("--proxy {}://{}:{}", s.scheme, s.host, s.port)
    }

    /// Returns just "scheme://host:port" (the proxy URL portion, no CLI flag).
    pub fn to_proxy_url() -> Option<String> {
        let s = state().lock().unwrap();
        if !s.enabled || s.host.is_empty() {
            return None;
        }
        Some(format!("{}://{}:{}", s.scheme, s.host, s.port))
    }

    /// Build a reqwest::Proxy from the current configuration.
    pub fn to_reqwest_proxy() -> Option<reqwest::Proxy> {
        let s = state().lock().unwrap();
        if !s.enabled || s.host.is_empty() {
            return None;
        }
        // Honor the configured scheme (http/https/socks5/...). Previously this
        // hard-coded "http://", so SOCKS5 proxies were silently tunneled via
        // HTTP CONNECT here while yt-dlp used the real socks5 scheme.
        let url = format!("{}://{}:{}", s.scheme, s.host, s.port);
        reqwest::Proxy::all(&url).ok()
    }

    // ==================== System Detection ====================

    /// Initialize proxy from JVM-like system properties and environment variables.
    /// Called once at startup. Does not override a manually-set proxy.
    pub fn init_from_environment() {
        {
            let s = state().lock().unwrap();
            if s.enabled {
                return; // already configured
            }
        }

        // 1. JVM-style system properties
        if let (Some(host), port) = (
            std::env::var("http.proxyHost").ok(),
            std::env::var("http.proxyPort").ok(),
        ) {
            if !host.is_empty() {
                let port: u16 = port.as_deref().unwrap_or("8080").parse().unwrap_or(8080);
                let mut s = state().lock().unwrap();
                s.host = host;
                s.port = port;
                s.enabled = true;
                return;
            }
        }

        if let (Some(host), port) = (
            std::env::var("https.proxyHost").ok(),
            std::env::var("https.proxyPort").ok(),
        ) {
            if !host.is_empty() {
                let port: u16 = port.as_deref().unwrap_or("8080").parse().unwrap_or(8080);
                let mut s = state().lock().unwrap();
                s.host = host;
                s.port = port;
                s.enabled = true;
                return;
            }
        }

        // 2. Environment variables: HTTP_PROXY / HTTPS_PROXY / http_proxy / https_proxy
        for var in &["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    if let Some((scheme, host, port)) = parse_proxy_url(&val) {
                        let mut s = state().lock().unwrap();
                        s.host = host;
                        s.port = port;
                        s.scheme = scheme;
                        s.enabled = true;
                        return;
                    }
                }
            }
        }
    }

    /// Detect Windows system proxy via Win32 API.
    /// Uses WinHttpGetIEProxyConfigForCurrentUser to get the real system proxy.
    /// Falls back to direct registry read if the WinHTTP API fails.
    /// Returns true if a proxy was detected and applied.
    pub fn detect_system_proxy() -> bool {
        if cfg!(not(windows)) {
            return false;
        }

        {
            let s = state().lock().unwrap();
            if s.from_system_proxy {
                return true; // already detected
            }
        }

        // Call Win32 API to read IE proxy config
        let result = detect_system_proxy_raw();
        tracing::info!("[XDownload] detect_system_proxy via WinHTTP: result={:?}", result);

        match result {
            Some((host, port)) => {
                let mut s = state().lock().unwrap();
                s.host = host;
                s.port = port;
                s.enabled = true;
                s.from_system_proxy = true;
                tracing::info!("detected system proxy via WinHTTP: {}:{}", s.host, s.port);
                true
            }
            None => {
                tracing::info!("[XDownload] WinHTTP detection returned no proxy, trying registry fallback...");
                // Fallback: read proxy settings directly from Windows Registry
                match Self::detect_system_proxy_from_registry() {
                    Some((host, port)) => {
                        let mut s = state().lock().unwrap();
                        s.host = host;
                        s.port = port;
                        s.enabled = true;
                        s.from_system_proxy = true;
                        tracing::info!("detected system proxy via registry: {}:{}", s.host, s.port);
                        true
                    }
                    None => {
                        tracing::info!("[XDownload] no system proxy detected via registry either");
                        false
                    }
                }
            }
        }
    }

    /// Fallback: read proxy settings directly from Windows Registry.
    /// Reads HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings
    /// — the same source that WinHTTP reads from.
    fn detect_system_proxy_from_registry() -> Option<(String, u16)> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey(
                r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            )
            .ok()?;

        // Check ProxyEnable (REG_DWORD): 0 = disabled, 1 = enabled
        let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
        if enabled == 0 {
            tracing::info!(
                "[XDownload] registry: ProxyEnable is 0, no system proxy configured"
            );
            return None;
        }

        // Read ProxyServer (REG_SZ): e.g. "127.0.0.1:7890" or "http=127.0.0.1:7890;https=..."
        let server: String = key.get_value("ProxyServer").ok()?;
        tracing::info!("[XDownload] registry: ProxyServer={}", server);

        if server.is_empty() {
            return None;
        }

        // Parse: may be "host:port" or "http=host:port;https=host:port"
        let host_port = if server.contains('=') {
            server
                .split(';')
                .find_map(|part| part.splitn(2, '=').nth(1).map(|v| v.trim()))
                .unwrap_or(&server)
                .to_string()
        } else {
            server
        };

        tracing::info!("[XDownload] registry: parsed host_port={}", host_port);
        parse_host_port(&host_port)
    }

    // ==================== Proxy Testing ====================

    /// Test whether a given proxy (host / port / scheme) can reach x.com.
    /// This is a **pure** connectivity test: it does NOT touch the global proxy
    /// state and does NOT persist anything. Returns a ProxyTestResult with
    /// success, HTTP status, elapsed_ms, and message.
    pub async fn test_proxy_config(host: &str, port: u16, scheme: &str) -> ProxyTestResult {
        if host.is_empty() || port == 0 {
            tracing::warn!(
                "[XDownload] test_proxy_config skipped: proxy not configured (host={} port={})",
                host,
                port
            );
            return ProxyTestResult::new(false, -1, 0, "proxy not configured".to_string());
        }
        let proxy_url = format!("{}://{}:{}", scheme, host, port);
        let proxy = match reqwest::Proxy::all(&proxy_url) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("[XDownload] test_proxy_config invalid proxy url {}: {e}", proxy_url);
                return ProxyTestResult::new(false, -1, 0, format!("invalid proxy url: {}", e));
            }
        };

        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(8))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[XDownload] test_proxy_config failed to build client: {e}");
                return ProxyTestResult::new(false, -1, 0, format!("failed to build client: {}", e));
            }
        };

        let start = std::time::Instant::now();
        match client.head("https://x.com").send().await {
            Ok(resp) => {
                let elapsed = start.elapsed().as_millis() as u64;
                let code = resp.status().as_u16() as i32;
                if code >= 200 && code < 400 {
                    tracing::info!(
                        "[XDownload] test_proxy_config OK: proxy={} status={} elapsed_ms={}",
                        proxy_url,
                        code,
                        elapsed
                    );
                    ProxyTestResult::new(true, code, elapsed, "proxy OK, x.com reachable".to_string())
                } else {
                    tracing::warn!(
                        "[XDownload] test_proxy_config unexpected status: proxy={} status={}",
                        proxy_url,
                        code
                    );
                    ProxyTestResult::new(
                        false,
                        code,
                        elapsed,
                        format!("x.com returned unexpected status: {}", code),
                    )
                }
            }
            Err(e) => {
                let elapsed = start.elapsed().as_millis() as u64;
                let msg = e.to_string();
                let message = if msg.contains("Connection refused") {
                    "connection refused, proxy port not open".to_string()
                } else if msg.contains("timeout") || msg.contains("Timeout") {
                    "connection timeout, proxy not responding".to_string()
                } else if msg.contains("dns") || msg.contains("resolve") {
                    "cannot resolve x.com, check DNS / proxy".to_string()
                } else {
                    format!("proxy connection failed: {}", msg)
                };
                tracing::warn!(
                    "[XDownload] test_proxy_config failed: proxy={} elapsed_ms={} reason={}",
                    proxy_url,
                    elapsed,
                    message
                );
                ProxyTestResult::new(false, -1, elapsed, message)
            }
        }
    }

    /// Test the currently configured proxy (reads the global state).
    pub async fn test_proxy() -> ProxyTestResult {
        let (scheme, host, port) = {
            let s = state().lock().unwrap();
            if !s.enabled || s.host.is_empty() {
                return ProxyTestResult::new(false, -1, 0, "proxy not enabled".to_string());
            }
            (s.scheme.clone(), s.host.clone(), s.port)
        };
        Self::test_proxy_config(&host, port, &scheme).await
    }
}

// ==================== Helpers ====================

/// Parse a proxy URL like "socks5://host:port" or "host:port" into
/// (scheme, host, port). Scheme defaults to "http" when absent.
fn parse_proxy_url(url: &str) -> Option<(String, String, u16)> {
    let trimmed = url.trim().trim_end_matches('/');
    let (scheme, rest) = match trimmed.find("://") {
        Some(idx) => (trimmed[..idx].to_lowercase(), &trimmed[idx + 3..]),
        None => ("http".to_string(), trimmed),
    };
    let (host, port) = parse_host_port(rest)?;
    Some((scheme, host, port))
}

/// Parse "host:port" into (host, port).
fn parse_host_port(s: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = s.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let host = parts[1].to_string();
    let port: u16 = parts[0].parse().ok()?;
    if host.is_empty() {
        return None;
    }
    Some((host, port))
}

// ==================== Helpers ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_port() {
        assert_eq!(
            parse_host_port("127.0.0.1:7890"),
            Some(("127.0.0.1".to_string(), 7890))
        );
        assert_eq!(
            parse_host_port("localhost:8080"),
            Some(("localhost".to_string(), 8080))
        );
        assert_eq!(parse_host_port("invalid"), None);
    }

    #[test]
    fn test_parse_proxy_url() {
        assert_eq!(
            parse_proxy_url("http://127.0.0.1:7890"),
            Some(("http".to_string(), "127.0.0.1".to_string(), 7890))
        );
        assert_eq!(
            parse_proxy_url("https://proxy.example.com:3128/"),
            Some(("https".to_string(), "proxy.example.com".to_string(), 3128))
        );
        assert_eq!(
            parse_proxy_url("socks5://127.0.0.1:1080"),
            Some(("socks5".to_string(), "127.0.0.1".to_string(), 1080))
        );
        assert_eq!(
            parse_proxy_url("10.0.0.1:1080"),
            Some(("http".to_string(), "10.0.0.1".to_string(), 1080))
        );
    }
}

// ==================== Win32 system proxy detection ====================

#[cfg(windows)]
mod sys_proxy {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::core::PWSTR;
    use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
    use windows::Win32::Networking::WinHttp::{
        WinHttpGetIEProxyConfigForCurrentUser, WINHTTP_CURRENT_USER_IE_PROXY_CONFIG,
    };

    /// Convert a PWSTR to Option<String> safely.
    unsafe fn pwstr_to_option_string(s: PWSTR) -> Option<String> {
        if s.is_null() {
            return None;
        }
        let mut len = 0;
        while *s.0.add(len) != 0 {
            len += 1;
        }
        if len == 0 {
            return None;
        }
        let slice = std::slice::from_raw_parts(s.0, len);
        Some(OsString::from_wide(slice).to_string_lossy().into_owned())
    }

    /// Call WinHttpGetIEProxyConfigForCurrentUser and return (host, port) if a proxy is enabled.
    /// Returns None if no proxy or detection failed.
    pub fn detect() -> Option<(String, u16)> {
        unsafe {
            tracing::info!("[XDownload] sys_proxy::detect: calling WinHttpGetIEProxyConfigForCurrentUser...");
            // Use std::mem::zeroed() for explicit zero-initialization (more
            // reliable than Default::default() in optimized release builds).
            let mut config: WINHTTP_CURRENT_USER_IE_PROXY_CONFIG = std::mem::zeroed();
            match WinHttpGetIEProxyConfigForCurrentUser(&mut config) {
                Ok(()) => {
                    tracing::info!("[XDownload] sys_proxy::detect: WinHttp API call succeeded");
                }
                Err(e) => {
                    tracing::info!(
                        "[XDownload] sys_proxy::detect: WinHttp API failed: {:?}",
                        e
                    );
                    return None;
                }
            }

            tracing::info!(
                "[XDownload] sys_proxy::detect: fAutoDetect={:?}, lpszAutoConfigUrl is_null={}, lpszProxy is_null={}",
                config.fAutoDetect,
                config.lpszAutoConfigUrl.is_null(),
                config.lpszProxy.is_null(),
            );

            let proxy = pwstr_to_option_string(config.lpszProxy);
            tracing::info!("[XDownload] sys_proxy::detect: raw proxy={:?}", proxy);

            // Free WinHttp-allocated strings
            if !config.lpszProxy.is_null() {
                let _ = GlobalFree(Some(HGLOBAL(config.lpszProxy.0 as _)));
            }
            if !config.lpszProxyBypass.is_null() {
                let _ = GlobalFree(Some(HGLOBAL(config.lpszProxyBypass.0 as _)));
            }
            if !config.lpszAutoConfigUrl.is_null() {
                let _ = GlobalFree(Some(HGLOBAL(config.lpszAutoConfigUrl.0 as _)));
            }

            // Parse: may be "host:port" or "http=host:port;https=host:port"
            let server = proxy?;
            if server.is_empty() {
                return None;
            }

            let host_port = if server.contains('=') {
                server
                    .split(';')
                    .find_map(|part| part.splitn(2, '=').nth(1).map(|v| v.trim()))
                    .unwrap_or(&server)
                    .to_string()
            } else {
                server
            };

            tracing::info!("[XDownload] sys_proxy::detect: parsed host_port={:?}", host_port);
            let result = super::parse_host_port(&host_port);
            tracing::info!("[XDownload] sys_proxy::detect: parse_host_port result={:?}", result);
            result
        }
    }
}

#[cfg(windows)]
use sys_proxy::detect as detect_system_proxy_raw;

#[cfg(not(windows))]
fn detect_system_proxy_raw() -> Option<(String, u16)> {
    None
}
