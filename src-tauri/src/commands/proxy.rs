use crate::services::proxy::ProxyConfig;

/// Test proxy connectivity to x.com
#[tauri::command]
pub async fn test_proxy(
    host: String,
    port: u32,
    scheme: Option<String>,
) -> Result<serde_json::Value, String> {
    let scheme = scheme.unwrap_or_else(|| "http".to_string());
    // Temporarily set the proxy for testing
    let port_u16 = port.min(65535) as u16;
    ProxyConfig::set_proxy_full(&host, port_u16, &scheme);

    let result = ProxyConfig::test_proxy().await;

    if result.success {
        // Save on successful test
        let _ = crate::services::config::ConfigManager::save_proxy(&host, port, &scheme);
    }

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
        // Re-enable from config
        crate::services::config::ConfigManager::apply_saved_proxy();
    } else {
        ProxyConfig::disable();
    }
}
