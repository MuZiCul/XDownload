/**
 * XDownload 下载助手 — content script
 *
 * 在 X/Twitter 页面的视频上叠加「下载」按钮，点击后通过自定义协议
 * `xdownload://add?url=<encoded status url>` 将视频加入 XDownload
 * 桌面应用的任务队列（应用内 yt-dlp 自动解析下载）。
 *
 * status URL 提取策略：
 *   1. 页面 URL 匹配 `/status/<id>` → 直接使用当前链接；
 *   2. 信息流/详情页内嵌视频 → 向上找 `article[data-testid="tweet"]`
 *      中的 `/status/<id>` 链接。
 */
(() => {
  // ============================================================
  // 书签 queryId 打印（来自 background 的 webRequest 观察）
  //
  // background.js 通过 chrome.webRequest 观察书签 GraphQL 请求
  // URL（只读，完全不影响页面），捕获 queryId 后：
  //   1. 主动向本脚本推送（chrome.tabs.sendMessage）
  //   2. 本脚本也定期轮询 background，覆盖推送丢失/注入较晚的情况
  //
  // 注意：绝不能在这里改写页面 fetch/XMLHttpRequest —— X 对书签
  // 等私有页面有反篡改检测，hook 会被识别并拒绝加载数据。
  // ============================================================
  function logQueryId(qid) {
    if (!qid) return;
    console.log("[XDownload] Bookmarks queryId =", qid, "(via extension)");
  }

  let reported = new Set();
  function reportFrom(resp) {
    if (resp && Array.isArray(resp.queryIds)) {
      resp.queryIds.forEach((qid) => {
        if (qid && !reported.has(qid)) {
          reported.add(qid);
          logQueryId(qid);
        }
      });
    }
  }

  try {
    chrome.runtime.onMessage.addListener((msg) => {
      if (msg && msg.type === "xdl-bookmarks-queryid") {
        reportFrom({ queryIds: [msg.queryId] });
      }
    });
  } catch (e) {
    // ignore
  }

  // 定期轮询 background：content script 可能在书签请求之后才注入
  // （document_idle），此时需主动取回已捕获的 queryId。
  let pollCount = 0;
  const POLL_MAX = 10; // ~20s
  (function pollQueryId() {
    try {
      chrome.runtime.sendMessage({ type: "xdl-get-queryid" }, reportFrom);
    } catch (e) {
      // ignore
    }
    if (++pollCount < POLL_MAX) {
      setTimeout(pollQueryId, 2000);
    }
  })();

  const PROTOCOL_PREFIX = "xdownload://add?url=";
  const BUTTON_DATA_ATTR = "data-xdl-dl";
  const BUTTON_ID_PREFIX = "xdl-btn-";
  const MAX_BUTTONS = 50;

  // 页面当前是否处于 /status/<id> 单推文页。
  function pageStatusUrl() {
    const m = location.pathname.match(/\/status\/(\d+)/);
    if (!m) return null;
    // 使用干净的 status 链接（去掉可能的后缀如 /video/1）
    return `${location.origin}${location.pathname}`.split("/video")[0];
  }

  // 从推文 article 中提取 status 链接。
  function statusUrlFromArticle(article) {
    if (!article || !article.querySelector("video")) return null;
    const link = article.querySelector('a[href*="/status/"]');
    if (!link) return null;
    const href = link.getAttribute("href");
    if (!href) return null;
    const m = href.match(/\/status\/(\d+)/);
    if (!m) return null;
    return `${location.origin}/${href.replace(/^\//, "").split("?")[0].split("/video")[0]}`;
  }

  function findStatusUrl(video) {
    const pageUrl = pageStatusUrl();
    if (pageUrl) return pageUrl;
    // 向上找最近的 article（推文容器）
    let node = video;
    while (node && node !== document.body) {
      if (node.matches && node.matches('article[data-testid="tweet"]')) {
        const u = statusUrlFromArticle(node);
        if (u) return u;
        break;
      }
      node = node.parentElement;
    }
    // 兜底：向上找任意含 /status/ 的链接
    node = video;
    while (node && node !== document.body) {
      if (node.querySelector) {
        const link = node.querySelector('a[href*="/status/"]');
        if (link) {
          const m = (link.getAttribute("href") || "").match(/\/status\/(\d+)/);
          if (m) {
            return `${location.origin}/${link
              .getAttribute("href")
              .replace(/^\//, "")
              .split("?")[0]
              .split("/video")[0]}`;
          }
        }
      }
      node = node.parentElement;
    }
    return null;
  }

  function sendToXDownload(statusUrl) {
    const url = PROTOCOL_PREFIX + encodeURIComponent(statusUrl);
    try {
      // 顶层导航触发自定义协议（iframe 内跳转无效）
      if (window.top === window.self) {
        window.location.href = url;
      } else {
        window.top.location.href = url;
      }
    } catch (e) {
      // 跨域限制时忽略
    }
  }

  function createButton(video) {
    if (video.hasAttribute(BUTTON_DATA_ATTR)) return;
    video.setAttribute(BUTTON_DATA_ATTR, "1");

    const btn = document.createElement("button");
    btn.id = BUTTON_ID_PREFIX + Math.random().toString(36).slice(2, 8);
    btn.textContent = "下载";
    btn.title = "加入 XDownload 下载队列";
    btn.style.cssText = [
      "position:absolute",
      "top:8px",
      "right:8px",
      "z-index:2147483647",
      "display:flex",
      "align-items:center",
      "gap:4px",
      "padding:5px 12px",
      "font-size:13px",
      "font-weight:600",
      "color:#fff",
      "background:rgba(29,155,240,0.95)",
      "border:none",
      "border-radius:9999px",
      "cursor:pointer",
      "box-shadow:0 2px 8px rgba(0,0,0,0.3)",
      "font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif",
    ].join(";");

    // 播放器可能有覆盖层，确保按钮可点击
    btn.addEventListener("click", (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      // 5 秒冷却中：忽略重复点击。
      if (btn.disabled) return;
      const statusUrl = findStatusUrl(video);
      if (!statusUrl) {
        flashButton(btn, "未找到链接");
        return;
      }
      // 进入等待：禁用按钮 5 秒，提示正在添加任务，避免连续误触。
      btn.disabled = true;
      btn.textContent = "正在添加任务";
      btn.style.opacity = "0.6";
      btn.style.cursor = "default";
      sendToXDownload(statusUrl);
      clearTimeout(btn._xdlCooldown);
      btn._xdlCooldown = setTimeout(() => {
        btn.disabled = false;
        btn.textContent = "下载";
        btn.style.opacity = "1";
        btn.style.cursor = "pointer";
      }, 5000);
    });

    // 挂载到 video 的外层（播放器容器），找不到则用 video 父级
    let host = video.parentElement;
    if (!host) return;
    // 确保 host 是相对定位，按钮才能绝对定位到播放器右上角
    const style = window.getComputedStyle(host);
    if (style.position === "static") host.style.position = "relative";
    host.appendChild(btn);

    return btn;
  }

  // 按钮级短提示（用于「未找到链接」等瞬时反馈），每个按钮独立 timer。
  function flashButton(btn, text) {
    btn.textContent = text;
    clearTimeout(btn._xdlFlash);
    btn._xdlFlash = setTimeout(() => {
      btn.textContent = "下载";
    }, 1200);
  }

  let injected = 0;
  function scan() {
    const videos = document.querySelectorAll("video");
    for (const v of videos) {
      if (injected >= MAX_BUTTONS) break;
      if (v.hasAttribute(BUTTON_DATA_ATTR)) continue;
      // 跳过过小的内嵌/头像视频（可能是预览图占位）
      if (v.offsetWidth < 100 || v.offsetHeight < 60) continue;
      if (createButton(v)) injected += 1;
    }
  }

  // 页面动态加载视频 → 持续扫描
  const observer = new MutationObserver(() => {
    scan();
  });
  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
  });

  // 首次扫描（document_idle 时视频可能已就位）
  setTimeout(scan, 500);
})();
