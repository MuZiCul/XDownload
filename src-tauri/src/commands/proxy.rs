use crate::services::proxy::ProxyConfig;

/// Test proxy connectivity to x.com
#[tauri::command]
pub async fn test_proxy(
    host: String,
    port: u32,
    scheme: Option<String>,
) -> Result<serde_json::Value, String> {
    let scheme = scheme.unwrap_or_else(|| "http".to_string());

    // Pure connectivity test for the given host/port/scheme. It does NOT touch
    // the runtime proxy state and does NOT persist anything — applying and
    // saving the proxy are handled explicitly by the frontend (save flow).
    let port_u16 = port.min(65535) as u16;
    tracing::info!(
        "[XDownload] test_proxy: host={} port={} scheme={}",
        host,
        port_u16,
        scheme
    );
    let result = ProxyConfig::test_proxy_config(&host, port_u16, &scheme).await;
    tracing::info!(
        "[XDownload] test_proxy result: success={} http_status={} elapsed_ms={} message={}",
        result.success,
        result.http_status,
        result.elapsed_ms,
        result.message
    );

    Ok(serde_json::json!({
        "success": result.success,
        "http_status": result.http_status,
        "elapsed_ms": result.elapsed_ms,
        "message": result.message,
    }))
}

/// Get current proxy status
#[tauri::command]
pub fn get_proxy_status() -> serde_json::Value {
    let enabled = ProxyConfig::is_enabled();
    let host = ProxyConfig::get_proxy_host();
    let port = ProxyConfig::get_proxy_port();
    let from_system = ProxyConfig::is_from_system_proxy();
    let proxy_string = ProxyConfig::get_proxy_string();
    tracing::info!(
        "[XDownload] get_proxy_status: enabled={} host={} port={} from_system={} proxy_string={}",
        enabled, host, port, from_system, proxy_string
    );
    serde_json::json!({
        "enabled": enabled,
        "host": host,
        "port": port,
        "from_system": from_system,
        "proxy_string": proxy_string,
    })
}

/// Set proxy mode (enable/disable manual proxy)
#[tauri::command]
pub fn set_proxy_mode(enabled: bool) {
    if enabled {
        // 系统代理曾检测到且仍启用 → 仅重新启用（保留系统来源标记），
        // 否则走 config 应用手动代理。修复"系统代理关闭后重开无法恢复"。
        if ProxyConfig::is_from_system_proxy() {
            ProxyConfig::enable();
        } else {
            crate::services::config::ConfigManager::apply_saved_proxy();
        }
    } else {
        ProxyConfig::disable();
        // 代理关闭时，「工具（yt-dlp/ffmpeg）下载走代理」开关必须同步关闭并持久化，
        // 避免后端配置与运行时状态不一致导致后续工具下载报"未配置代理"。
        if let Err(e) = crate::services::config::ConfigManager::save_tools_use_proxy(false) {
            tracing::warn!(
                "[XDownload] set_proxy_mode: failed to persist tools_use_proxy=false: {}",
                e
            );
        }
    }
}

/// Apply manual proxy values (host/port/scheme) to runtime, overriding any
/// existing system proxy. Used when the user saves manual proxy mode.
#[tauri::command]
pub fn apply_manual_proxy(host: String, port: u16, scheme: String) -> Result<(), String> {
    crate::services::config::ConfigManager::apply_manual_proxy(&host, port, &scheme);
    Ok(())
}
