//! 日志文件读取工具（供 `get_logs` 命令使用）。
//!
//! 原浏览器版日志查看器（本地 HTTP 服务 + 内嵌页面）已由应用内 React 日志页
//! 替代，这里仅保留被 `commands::logs::get_logs` 复用的文件读取能力。

use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::fs::File;

/// 单次读取文件的最大字节数（避免大文件整读卡死页面）。
const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// 文件名安全校验：只允许 `xdownload.log.YYYY-MM-DD` 形式的文件名，防止
/// 路径穿越（`..`、分隔符）读取任意文件。
pub(crate) fn is_safe_log_name(name: &str) -> bool {
    name.starts_with("xdownload.log.")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// 列出 logs/ 下所有 `xdownload.log.*` 文件，按修改时间倒序（最新在前）。
pub(crate) async fn list_log_files(logs_dir: &Path) -> Vec<String> {
    let mut rd = match tokio::fs::read_dir(logs_dir).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut items = Vec::new();
    while let Ok(Some(entry)) = rd.next_entry().await {
        let Ok(ft) = entry.file_type().await else { continue };
        if !ft.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_safe_log_name(&name) {
            continue;
        }
        let mtime = entry
            .metadata()
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        items.push((mtime, name));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items.into_iter().map(|(_, n)| n).collect()
}

/// 读取日志文件末尾 `max_lines` 行（最多读取尾部 `MAX_BYTES` 字节），
/// 返回 (文件字节数, 行列表)。
pub(crate) async fn read_tail(logs_dir: &Path, name: &str, max_lines: usize) -> (u64, Vec<String>) {
    let path = logs_dir.join(name);
    let Ok(mut f) = File::open(&path).await else {
        return (0, Vec::new());
    };
    let Ok(meta) = f.metadata().await else {
        return (0, Vec::new());
    };
    let size = meta.len();
    let start = size.saturating_sub(MAX_BYTES);

    let mut buf = Vec::new();
    if start > 0 {
        if f.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return (size, Vec::new());
        }
    }
    if f.read_to_end(&mut buf).await.is_err() {
        return (size, Vec::new());
    }

    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    // 截断处首行通常不完整，丢弃以免显示半个字段。
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    if lines.len() > max_lines {
        lines.drain(..lines.len() - max_lines);
    }
    (size, lines)
}
