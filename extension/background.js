// XDownload download assistant - background service worker
//
// Watches X/Twitter network requests with the non-blocking webRequest API to
// capture the (rotating) Bookmarks GraphQL queryId. Reading the request URL
// does NOT modify the page in any way, so the X web app is never affected
// (unlike monkey-patching XMLHttpRequest/fetch, which X detects on its
// private pages such as /i/bookmarks and refuses to render data).
//
// The queryId is pushed to the matching tabs' content scripts via
// chrome.tabs.sendMessage (NOT chrome.runtime.sendMessage - that only reaches
// extension pages, never content scripts).

const BOOKMARKS_RE = /\/i\/api\/graphql\/([A-Za-z0-9_-]{8,60})\/Bookmarks/;
const SEEN = new Set();

function isTwitterTab(url) {
  return !!url && /^https:\/\/([^/]*\.)?(x|twitter)\.com\//.test(url);
}

function broadcast(qid) {
  try {
    chrome.tabs.query({}, (tabs) => {
      for (const t of tabs || []) {
        if (t.id != null && isTwitterTab(t.url || '')) {
          chrome.tabs.sendMessage(t.id, { type: 'xdl-bookmarks-queryid', queryId: qid }, () => {
            // Ignore "Receiving end does not exist" (no content script yet).
            void chrome.runtime.lastError;
          });
        }
      }
    });
  } catch (e) {
    // ignore
  }
}

function handleRequest(details) {
  const m = BOOKMARKS_RE.exec(details.url || '');
  if (!m) return;
  const qid = m[1];
  if (SEEN.has(qid)) return;
  SEEN.add(qid);
  console.log('[XDownload] background captured Bookmarks queryId =', qid);
  broadcast(qid);
}

try {
  chrome.webRequest.onBeforeRequest.addListener(
    handleRequest,
    { urls: ['*://x.com/*', '*://twitter.com/*'] },
  );
  console.log('[XDownload] webRequest listener installed');
} catch (e) {
  console.warn('[XDownload] webRequest listener failed:', e);
}

// Content scripts poll for any queryId already captured (they may inject
// after the first request was made, or a push may have been missed).
try {
  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg && msg.type === 'xdl-get-queryid') {
      sendResponse({ alive: true, queryIds: [...SEEN] });
      return false;
    }
    return undefined;
  });
} catch (e) {
  // ignore
}
