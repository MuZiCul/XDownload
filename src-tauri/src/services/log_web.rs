//! 本地日志查看 HTTP 服务。
//!
//! 点击设置里的「软件日志」按钮时，启动一个仅监听 `127.0.0.1` 随机端口的
//! 极简 HTTP 服务，返回一个内嵌的日志查看器页面（深色等宽、级别着色、按
//! 日期切换、2 秒自动刷新），再用系统默认浏览器打开。
//!
//! 设计取舍：
//! - 不引入任何新依赖：tokio "full" 自带 TcpListener / AsyncReadExt 等。
//! - 手写极简 HTTP/1.1：只支持 GET，读到一个请求响应后关闭连接。
//! - 服务随应用进程退出而自然消亡（监听 socket 随进程关闭，无需清理）。
//! - 只监听 127.0.0.1 + 系统随机端口，不暴露到局域网。
//! - 单例：服务只启动一次，之后点击直接复用端口。

use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// 单例端口：服务只会启动一次，后续点击直接复用。
static PORT: std::sync::Mutex<Option<u16>> = std::sync::Mutex::new(None);

/// 每次接口返回的最大行数。
const MAX_LINES: usize = 2000;
/// 单次读取文件的最大字节数（避免大文件整读卡死页面）。
const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// 启动日志查看服务（幂等）。返回访问端口。
pub async fn ensure_started() -> Result<u16, String> {
    if let Some(p) = *PORT.lock().map_err(|_| "log_web: port lock poisoned")? {
        return Ok(p);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("log_web: bind failed: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("log_web: local_addr failed: {}", e))?
        .port();
    *PORT.lock().unwrap() = Some(port);

    let logs_dir = crate::utils::app_home::AppHome::logs_dir();
    tokio::spawn(async move {
        serve(listener, logs_dir).await;
    });
    tracing::info!("log_web: viewer started at http://127.0.0.1:{}/", port);
    Ok(port)
}

/// 接受循环：每个连接独立 spawn 一个任务处理。
async fn serve(listener: TcpListener, logs_dir: std::path::PathBuf) {
    loop {
        let Ok((mut sock, _peer)) = listener.accept().await else {
            continue;
        };
        let logs_dir = logs_dir.clone();
        tokio::spawn(async move {
            let _ = handle_conn(&mut sock, &logs_dir).await;
            let _ = sock.shutdown().await;
        });
    }
}

/// 读取请求头（`\r\n\r\n` 结束），返回完整的请求目标（含 query）。
async fn read_request_target(sock: &mut TcpStream) -> Result<String, String> {
    let mut acc = Vec::with_capacity(1024);
    let mut buf = [0u8; 4096];
    loop {
        let n = sock
            .read(&mut buf)
            .await
            .map_err(|e| format!("log_web: read failed: {}", e))?;
        if n == 0 {
            return Err("log_web: connection closed".into());
        }
        acc.extend_from_slice(&buf[..n]);
        if acc.len() > 32 * 1024 {
            return Err("log_web: request too large".into());
        }
        if let Some(pos) = acc.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&acc[..pos]);
            let first = head.lines().next().unwrap_or("");
            let mut parts = first.split_whitespace();
            let method = parts.next().unwrap_or("");
            let target = parts.next().unwrap_or("/");
            if method != "GET" {
                return Err("log_web: method not allowed".into());
            }
            return Ok(target.to_string());
        }
    }
}

/// 处理单个 HTTP 请求：解析目标 → 生成响应 → 写出。
async fn handle_conn(sock: &mut TcpStream, logs_dir: &Path) -> Result<(), String> {
    let target = read_request_target(sock).await?;
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (target.as_str(), None),
    };

    let (status, status_text, content_type, body) = match path {
        "/" | "/index.html" => (200, "OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes().to_vec()),
        "/api/logs" => {
            let body = build_logs_json(logs_dir, query).await;
            (200, "OK", "application/json; charset=utf-8", body)
        }
        _ => (404, "Not Found", "text/plain; charset=utf-8", b"Not Found".to_vec()),
    };

    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        status, status_text, content_type, body.len()
    );
    sock.write_all(header.as_bytes())
        .await
        .map_err(|e| format!("log_web: write header failed: {}", e))?;
    sock.write_all(&body)
        .await
        .map_err(|e| format!("log_web: write body failed: {}", e))?;
    sock.flush()
        .await
        .map_err(|e| format!("log_web: flush failed: {}", e))
}

/// 解析 query 中的 `file` / `lines` 参数。
struct QueryParams {
    file: Option<String>,
    lines: usize,
}

fn parse_query(query: Option<&str>) -> QueryParams {
    let mut params = QueryParams {
        file: None,
        lines: MAX_LINES,
    };
    let Some(query) = query else {
        return params;
    };
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let (k, v) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
        match k {
            "file" => params.file = Some(v.to_string()),
            "lines" => {
                if let Ok(n) = v.parse::<usize>() {
                    params.lines = n.clamp(1, 20000);
                }
            }
            _ => {}
        }
    }
    params
}

/// 构建 `/api/logs` 的 JSON：文件列表 + 当前文件尾部内容。
async fn build_logs_json(logs_dir: &Path, query: Option<&str>) -> Vec<u8> {
    let params = parse_query(query);
    let files = list_log_files(logs_dir).await;

    // 选择目标文件：优先用户指定（校验安全），否则取最新的。
    let target = params
        .file
        .filter(|name| is_safe_log_name(name) && files.iter().any(|f| f == name))
        .or_else(|| files.first().cloned())
        .unwrap_or_default();

    let (size, tail) = if target.is_empty() {
        (0, Vec::new())
    } else {
        read_tail(logs_dir, &target, params.lines).await
    };

    let json = serde_json::json!({
        "files": files,
        "file": target,
        "size": size,
        "lines": tail.len(),
        "tail": tail,
    });
    serde_json::to_vec(&json).unwrap_or_else(|_| b"{}".to_vec())
}

/// 文件名安全校验：只允许 `xdownload.log.YYYY-MM-DD` 形式的文件名，防止
/// 路径穿越（`..`、分隔符）读取任意文件。
fn is_safe_log_name(name: &str) -> bool {
    name.starts_with("xdownload.log.")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// 列出 logs/ 下所有 `xdownload.log.*` 文件，按修改时间倒序（最新在前）。
async fn list_log_files(logs_dir: &Path) -> Vec<String> {
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
async fn read_tail(logs_dir: &Path, name: &str, max_lines: usize) -> (u64, Vec<String>) {
    let path = logs_dir.join(name);
    let Ok(mut f) = tokio::fs::File::open(&path).await else {
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
        if f.read_to_end(&mut buf).await.is_err() {
            return (size, Vec::new());
        }
    } else {
        if f.read_to_end(&mut buf).await.is_err() {
            return (size, Vec::new());
        }
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

/// 内嵌的日志查看器页面（深色等宽、级别着色、按日期切换、2 秒自动刷新）。
const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDownload 日志查看器</title>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: "Cascadia Code", "Consolas", "SFMono-Regular", monospace;
    background: #0f172a;
    color: #cbd5e1;
    font-size: 13px;
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: #1e293b;
    border-bottom: 1px solid #334155;
    flex-wrap: wrap;
  }
  h1 { font-size: 14px; margin: 0; color: #f8fafc; font-weight: 600; }
  header select {
    background: #0f172a;
    color: #e2e8f0;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 12px;
    font-family: inherit;
    max-width: 300px;
  }
  header label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #94a3b8;
    cursor: pointer;
  }
  header button {
    background: #2563eb;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
  }
  header button:hover { background: #1d4ed8; }
  #meta { margin-left: auto; font-size: 11px; color: #64748b; }
  main { flex: 1; overflow: auto; padding: 8px 0; }
  pre#log {
    margin: 0;
    padding: 0 14px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .line-error { color: #f87171; }
</style>
</head>
<body>
<header>
  <h1>XDownload 日志查看器</h1>
  <select id="file" title="按日期切换日志文件"></select>
  <select id="level" title="按日志等级筛选">
    <option value="">全部</option>
    <option value="ERROR">ERROR</option>
    <option value="WARN">WARN</option>
    <option value="INFO">INFO</option>
    <option value="DEBUG">DEBUG</option>
    <option value="TRACE">TRACE</option>
  </select>
  <label><input type="checkbox" id="autorefresh" checked> 自动刷新 (2s)</label>
  <button id="refresh">刷新</button>
  <button id="scrollTop">回到最新</button>
  <span id="meta"></span>
</header>
<main id="main">
  <pre id="log"></pre>
</main>
<script>
(function () {
  "use strict";
  var $log = document.getElementById("log");
  var $main = document.getElementById("main");
  var $file = document.getElementById("file");
  var $level = document.getElementById("level");
  var $auto = document.getElementById("autorefresh");
  var $meta = document.getElementById("meta");
  var currentFile = "";
  var follow = true;

  function esc(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  // 日志等级权重：数值越大越严重，用于">= 所选等级"的筛选。
  var LEVEL_ORDER = { ERROR: 4, WARN: 3, INFO: 2, DEBUG: 1, TRACE: 0 };

  function lineLevel(line) {
    var m = line.match(/\b(ERROR|WARN|INFO|DEBUG|TRACE)\b/);
    return m ? m[1] : null;
  }

  // 筛选：未选择（全部）或行级别 >= 所选级别时保留。
  function passLevel(line) {
    var sel = $level.value;
    if (!sel) { return true; }
    var lv = lineLevel(line);
    if (!lv) { return true; }
    return LEVEL_ORDER[lv] >= LEVEL_ORDER[sel];
  }

  // ANSI 转义码 → 样式的映射（颜色值适配深色主题）。
  var ANSI = {
    "1": "font-weight:700;",
    "2": "opacity:0.7;",
    "3": "font-style:italic;",
    "4": "text-decoration:underline;",
    "30": "color:#94a3b8;",
    "31": "color:#f87171;",
    "32": "color:#4ade80;",
    "33": "color:#fbbf24;",
    "34": "color:#60a5fa;",
    "35": "color:#e879f9;",
    "36": "color:#22d3ee;",
    "37": "color:#e2e8f0;",
    "90": "color:#64748b;",
    "91": "color:#f87171;",
    "92": "color:#4ade80;",
    "93": "color:#fbbf24;",
    "94": "color:#60a5fa;",
    "95": "color:#e879f9;",
    "96": "color:#22d3ee;",
    "97": "color:#f8fafc;"
  };

  // 把文本里的 ANSI 转义码（如 ESC[32m、ESC[0m）渲染成彩色 <span>。
  // 文本须先经 esc() 转义，避免插入标签被利用；ESC(0x1b) 不受转义影响。
  function ansiToHtml(s) {
    var out = "";
    var open = 0; // 已打开未闭合的 <span> 数
    var last = 0;
    var re = /\x1b\[([0-9;]*)m/g;
    var m;
    while ((m = re.exec(s)) !== null) {
      out += s.slice(last, m.index);
      var codes = m[1].split(";");
      var style = "";
      var reset = m[1] === "" || codes.indexOf("0") !== -1;
      for (var i = 0; i < codes.length; i++) {
        if (codes[i] !== "0" && ANSI[codes[i]]) { style += ANSI[codes[i]]; }
      }
      if (reset) {
        while (open > 0) { out += "</span>"; open--; }
      } else if (style) {
        out += '<span style="' + style + '">';
        open++;
      }
      last = re.lastIndex;
    }
    out += s.slice(last);
    while (open > 0) { out += "</span>"; open--; }
    return out;
  }

  function renderLine(line) {
    var html = ansiToHtml(esc(line));
    if (lineLevel(line) === "ERROR") {
      return '<span class="line-error">' + html + '</span>';
    }
    return html;
  }

  function fmtSize(n) {
    if (n >= 1048576) { return (n / 1048576).toFixed(2) + " MB"; }
    if (n >= 1024) { return (n / 1024).toFixed(1) + " KB"; }
    return n + " B";
  }

  function refreshFileOptions(files) {
    var cur = $file.value || currentFile;
    $file.innerHTML = "";
    files.forEach(function (f) {
      var opt = document.createElement("option");
      opt.value = f;
      opt.textContent = f.replace(/^xdownload\.log\./, "");
      $file.appendChild(opt);
    });
    if (files.length && cur) {
      $file.value = files.indexOf(cur) >= 0 ? cur : files[0];
    }
  }

  function load() {
    var q = new URLSearchParams({ lines: "2000" });
    if (currentFile) { q.set("file", currentFile); }
    return fetch("/api/logs?" + q.toString())
      .then(function (r) { if (!r.ok) { throw new Error("HTTP " + r.status); } return r.json(); })
      .then(function (data) {
        refreshFileOptions(data.files || []);
        if (!data.file) {
          $log.textContent = "（暂无日志文件）";
          $meta.textContent = "";
          return;
        }
        currentFile = data.file;
        var lines = data.tail || [];
        var html = "";
        var shown = 0;
        for (var i = 0; i < lines.length; i++) {
          if (!passLevel(lines[i])) { continue; }
          shown++;
          html += renderLine(lines[i]) + "\n";
        }
        $meta.textContent = data.file + " · " + fmtSize(data.size || 0) + " · 显示 " + shown + " / " + (data.lines || 0) + " 行";
        $log.innerHTML = html;
        if (follow) { $main.scrollTop = $main.scrollHeight; }
      })
      .catch(function (err) {
        $log.textContent = "加载日志失败：" + err;
      });
  }

  $file.addEventListener("change", function () {
    currentFile = $file.value;
    follow = true;
    load();
  });

  $level.addEventListener("change", function () {
    follow = true;
    load();
  });

  document.getElementById("refresh").addEventListener("click", load);
  document.getElementById("scrollTop").addEventListener("click", function () {
    follow = true;
    $main.scrollTop = $main.scrollHeight;
  });

  $main.addEventListener("scroll", function () {
    follow = $main.scrollHeight - $main.scrollTop - $main.clientHeight < 150;
  });

  load();
  setInterval(function () {
    if ($auto.checked && document.visibilityState !== "hidden") { load(); }
  }, 2000);
})();
</script>
</body>
</html>
"##;
