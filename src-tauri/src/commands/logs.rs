//! Log viewer commands — serve log file lists and line tail to the in-app
//! React log viewer.

use serde_json::json;

/// 返回日志文件列表 + 指定文件（或最新文件）的尾部内容。
///
/// 前端每次轮询（2s 自动刷新）调用本命令：
/// - `file`: 可选，目标日志文件名（如 `xdownload.log.2026-08-18`）
/// - `lines`: 可选，返回最大行数（默认 2000，上限 20000）
#[tauri::command]
pub async fn get_logs(
    file: Option<String>,
    lines: Option<usize>,
) -> Result<serde_json::Value, String> {
    let logs_dir = crate::utils::app_home::AppHome::logs_dir();
    let max_lines = lines.unwrap_or(2000).clamp(1, 20000);

    let files = crate::services::log_files::list_log_files(&logs_dir).await;

    // 选择目标文件：优先用户指定（校验安全），否则取最新的。
    let target = file
        .as_deref()
        .filter(|name| crate::services::log_files::is_safe_log_name(name) && files.iter().any(|f| f == name))
        .or_else(|| files.first().map(String::as_str))
        .unwrap_or_default()
        .to_string();

    let (size, tail) = if target.is_empty() {
        (0, Vec::new())
    } else {
        crate::services::log_files::read_tail(&logs_dir, &target, max_lines).await
    };

    Ok(json!({
        "files": files,
        "file": target,
        "size": size,
        "lines": tail.len(),
        "tail": tail,
    }))
}
