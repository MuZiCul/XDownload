// XDownload popup: shows the captured Bookmarks queryId and lets the user
// push it to the desktop app via the xdownload://setqueryid deep link.
//
// Only the latest captured id is shown (SEEN keeps insertion order, so the
// last element is the newest). Pushing is manual on purpose — the deep link
// hands control to the desktop app, and the user decides when to do that.

(function () {
  "use strict";

  const statusEl = document.getElementById("qid-status");
  const valueEl = document.getElementById("qid-value");
  const pushBtn = document.getElementById("push-qid");
  const hintEl = document.getElementById("qid-hint");

  function render(qid) {
    if (qid) {
      statusEl.textContent = "已捕获（最近一次打开书签页时）";
      valueEl.textContent = qid;
      pushBtn.disabled = false;
      pushBtn.textContent = "推送 queryId 到桌面端";
      hintEl.textContent = "书签页";
    } else {
      statusEl.textContent = "尚未捕获";
      valueEl.textContent =
        "打开 x.com 书签页并刷新，扩展会自动捕获最新 queryId。";
      pushBtn.disabled = true;
      hintEl.textContent = "书签页";
    }
  }

  function refresh() {
    try {
      chrome.runtime.sendMessage({ type: "xdl-get-queryid" }, (resp) => {
        if (chrome.runtime.lastError) {
          render(null);
          return;
        }
        const qids = (resp && resp.queryIds) || [];
        render(qids.length ? qids[qids.length - 1] : null);
      });
    } catch (e) {
      render(null);
    }
  }

  pushBtn.addEventListener("click", () => {
    const qid = (valueEl.textContent || "").trim();
    if (!qid) return;
    pushBtn.disabled = true;
    pushBtn.textContent = "已触发推送，请回到桌面端确认";
    // 触发自定义协议，把 queryId 交给桌面端存库。若浏览器询问
    // 「打开 XDownload？」，勾选「始终允许」后下次将静默推送。
    try {
      window.location.href =
        "xdownload://setqueryid?value=" + encodeURIComponent(qid);
    } catch (e) {
      // ignore — 协议触发失败不影响扩展自身。
    }
  });

  refresh();
})();
