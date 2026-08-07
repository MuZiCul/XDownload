/**
 * English dictionary for the frontend UI.
 * Keys mirror zh.ts.
 */
export const en: Record<string, string> = {
  // ===== Tabs =====
  "tab.download": "Download",
  "tab.settings": "Settings",
  "tab.history": "History",
  "tab.about": "About",
  "tab.disclaimer": "Disclaimer",

  // ===== Global download bar =====
  "gbar.progressLabel": "Progress",
  "gbar.stageVideo": "Downloading [Video]",
  "gbar.stageAudio": "Downloading [Audio]",
  "gbar.stageMerge": "Merging",
  "gbar.downloading": "Downloading...",
  "gbar.cancel": "Cancel",
  "gbar.complete": "Download complete",
  "gbar.failed": "Download failed: {msg}",
  "gbar.close": "Close",

  // ===== URL bar =====
  "url.placeholder": "Paste X/Twitter video link...",
  "url.paste": "Paste",
  "url.fetch": "Fetch Info",
  "url.clipboard.empty": "Clipboard is empty",
  "url.clipboard.fail": "Unable to read clipboard, please paste manually",

  // ===== Format table =====
  "format.title": "Formats",
  "format.id": "ID",
  "format.ext": "Ext",
  "format.res": "Resolution",
  "format.size": "Size",

  // ===== Video info card =====
  "video.info": "Video Info",
  "video.author": "Author",
  "video.duration": "Duration",
  "video.views": "Views",
  "video.likes": "Likes",
  "video.multimedia":
    "This tweet contains {count} media items; all will be downloaded",
  "video.downloaded": "✓ Downloaded",
  "video.cancelDownload": "Cancel Download",
  "video.openPath": "Open File Location",
  "video.redownload": "Re-download",
  "video.startDownload": "Start Download",
  "video.repeatTitle": "Re-download",
  "video.repeatBody": "This video was already downloaded{time}. Re-download?",
  "video.openPathFail": "Failed to open file location: {err}",
  "video.openInBrowser": "Open in browser",
  "video.openUrlFail": "Failed to open in browser: {err}",
  "num.billion": "B",
  "num.tenThousand": "K",

  // ===== Download page / tools =====
  "tools.missing.ytdlp":
    "yt-dlp is not installed. Please download it from Tools in Settings.",
  "tools.missing.ffmpeg":
    "ffmpeg is not installed. Please download it from Tools in Settings.",
  "tools.ffmpegNotBundled":
    "Bundled ffmpeg not found; highest quality (video+audio merge) may be unavailable. Download it from Settings > Tools.",
  "prog.fetching": "Fetching video info...",
  "url.fetchOk": "Success",
  "url.fetchFail": "Fetch failed: {err}",

  // ===== Friendly error messages (errorMessages.ts) =====
  "error.suspended": "This video's author has been suspended on X; content unavailable",
  "error.private": "This account is private/protected; log in and follow to view",
  "error.deleted": "This tweet has been deleted or is unavailable",
  "error.nsfw": "This content requires login or age verification",
  "error.noVideo": "No downloadable video found in this tweet",
  "error.guestToken": "Failed to obtain a guest identity; try setting Cookies or a proxy",
  "error.rateLimit": "Too many requests, please try again later",
  "error.geoblocked": "This content is not available in your region",
  "error.broadcast": "This broadcast has ended or no longer exists",
  "error.space": "This Space does not exist or has ended",
  "error.api": "X API returned an error, please try again later",

  // ===== Common =====
  "common.cancel": "Cancel",
  "common.loading": "Loading...",
  "common.save": "Save",
  "common.saving": "Saving...",
  "common.saveFail": "Save failed: {err}",
  "common.openFail": "Open failed: {err}",
  "common.applyFail": "Apply failed: {err}",
  "common.quitFail": "Quit failed: {err}",
  "common.close": "Close",

  // ===== Directory setting =====
  "dir.title": "Download Directory",
  "dir.browse": "Browse",
  "dir.open": "Open",
  "dir.saved": "Download directory saved",

  // ===== Proxy setting =====
  "proxy.title": "Proxy",
  "proxy.none": "None",
  "proxy.manual": "Manual",
  "proxy.system": "System",
  "proxy.type": "Type:",
  "proxy.host": "Host:",
  "proxy.port": "Port:",
  "proxy.test": "Test",
  "proxy.testing": "● Testing...",
  "proxy.ok": "● OK",
  "proxy.error": "● Proxy error",
  "proxy.noSystem":
    "No system proxy detected. Enable one or use manual proxy.",
  "proxy.hostRequired": "Please enter a proxy host",
  "proxy.testPassed": "Proxy OK ({ms}ms)",
  "proxy.testFail": "Proxy test failed: {msg}",
  "proxy.disabledSaved": "Proxy disabled and saved",
  "proxy.savedApplied": "Proxy saved and applied",

  // ===== Cookies setting =====
  "cookies.browser": "Browser:",
  "cookies.none": "none",
  "cookies.validate": "Validate",
  "cookies.validatingBtn": "Validating...",
  "cookies.saveAndApply": "Save & Load",
  "cookies.statusValidating": "Validating: {browser}",
  "cookies.statusVerified": "Verified: {browser}{user} — save to apply",
  "cookies.statusLoaded": "Loaded: {browser}{user}",
  "cookies.statusNone": "No cookies",
  "cookies.saved": "Cookies saved and loaded: {browser}",
  "cookies.step1": "Extracting cookies from {browser}...",
  "cookies.step2": "x.com auth_token found, verifying login...",
  "cookies.step3": "x.com login valid",
  "cookies.verifiedOk": "Cookies valid, logged in as {user}",
  "cookies.error.browser_locked":
    "{browser} is running and its cookie database is locked. Close the browser and retry.",
  "cookies.error.browser_not_found":
    "{browser} cookie database not found (browser not installed or never used)",
  "cookies.error.no_auth_token":
    "No x.com auth_token found in browser cookies. Make sure you're logged in to x.com.",
  "cookies.error.token_invalid":
    "auth_token expired or invalid. Please log in to x.com again.",
  "cookies.error.network": "Cannot reach x.com: {msg}",
  "cookies.error.timeout": "{browser} validation timed out",
  "cookies.error.parse":
    "Could not parse a username from x.com; the page structure may have changed",
  "cookies.error.unknown": "Cookies validation failed: {msg}",

  // ===== Config buttons =====
  "config.save": "Save Config",
  "config.apply": "Apply Config",
  "config.dir": "Config Dir",
  "config.dirTitle": "Open the config folder under the root",
  "config.quit": "Quit",
  "config.quitTitle": "Clean up processes and quit",
  "config.path": "Config: {path}",
  "config.exported": "Config exported\nPath: {path}",
  "config.saved": "Config saved\nPath: {path}",
  "config.imported": "Config imported and persisted\nSource: {path}",
  "config.restored": "Restored default config\nPath: {path}",
  "config.dirOpened": "Config directory opened",
  "config.saveDialogTitle": "Choose Save Location",
  "config.saveDialogBody":
    "Default saves to the app config; custom exports elsewhere",
  "config.saveDefault": "Default Directory (app config)",
  "config.saveCustom": "Custom Directory (export)",
  "config.applyDialogTitle": "Choose Config Source",
  "config.applyDialogBody":
    "Default restores factory settings; custom imports from file",
  "config.applyDefault": "Restore Defaults",
  "config.applyCustom": "Custom Directory (import)",

  // ===== Tools setting =====
  "tools.checkFail": "Check failed: {err}",
  "tools.rootOpened": "Root directory opened",
  "tools.statusCheckFailed": "Check failed",
  "tools.statusNotInstalled": "Not installed",
  "tools.statusCurrent": "Current {ver}",
  "tools.statusUnknown": "Version unknown",
  "tools.statusLatest": " → Latest {ver}",
  "tools.downloadBtn": "Download",
  "tools.updateBtn": "Update",
  "tools.latest": "✓ Up to date",
  "tools.guideTitle": "Download Guide",
  "tools.checking": "Checking...",
  "tools.checkUpdate": "Check Updates",
  "tools.checkingNetwork": "Checking network connection",
  "tools.networkFail": "Cannot reach the download server",
  "tools.networkFailBody":
    "Check your network or configure a proxy.\nStill try to download {tool}?",
  "tools.continueDownload": "Continue",
  "tools.extracting": "Extracting ffmpeg",
  "tools.extractingDetail":
    "Extracting ffmpeg.exe / ffprobe.exe / ffplay.exe",
  "tools.downloading": "Downloading {tool}",
  "tools.ffmpegSize": "(~80MB, ~150MB after extraction)",
  "tools.ytdlpSize": "(~15MB)",
  "tools.connecting": "Connecting...",
  "tools.cancelDownload": "Cancel Download",
  "tools.downloadDone": "{tool} downloaded",
  "tools.guideTip":
    "If downloads are slow or fail, configure a proxy first.\nDomestic users should enable a proxy.\nYou can also download manually and extract to the bin/ folder.",
  "tools.ytdlpDesc": "Video extraction & download engine · ~15MB",
  "tools.ffmpegDesc": "Audio/video merge & transcode · ~80MB (~150MB extracted)",
  "tools.rootDir": "Root Dir",
  "tools.checkResultTitle": "Update Check Result",

  // ===== Language setting =====
  "lang.title": "Language",
  "lang.saved": "Language saved: {lang}",
  "lang.zh": "Chinese",
  "lang.en": "English",
  "lang.hintImmediate": "Takes effect immediately",

  // ===== About page =====
  "about.desc": "Video downloader based on yt-dlp",
  "about.checking": "Checking...",
  "about.checkUpdate": "Check Update",
  "about.newVersion": "New version v{ver} found",
  "about.currentVersion": "Current version v{ver}",
  "about.goDownload": "Go Download",
  "about.upToDate": "You're up to date",
  "about.checkFail": "Check failed, please check your network",

  // ===== History page =====
  "history.title": "Download History",
  "history.count": "{count} records",
  "history.clearAll": "Clear All",
  "history.empty": "No download history",
  "history.fileDeleted": "File deleted",
  "history.downloadedAt": "Downloaded at",
  "history.noUrl": "This record has no original URL. Fetch the video info first and try again.",
  "history.busy": "A download is already in progress. Please wait for it to finish.",
  "history.open": "Open",
  "history.delete": "Delete",
  "history.cleared": "Download history cleared",
  "history.deleteFail": "Delete failed: {err}",
  "history.clearFail": "Clear failed: {err}",

  // ===== App update modal =====
  "app.toolNotInstalled":
    "Please download {label} in Settings first, otherwise download won't work",
  "app.currentVersion": "Current v{ver}",
  "app.downloadFail": "Update download failed: {err}",
  "app.installFail": "Install failed: {err}",
  "app.newVersion": "New version available",
  "app.toolStatus": "Tool Status",
  "app.downloadUpdate": "Download Update",
  "app.download": "Download",
  "app.installUpdate": "Install Update",
  "app.installing": "Installing, the app will exit automatically...",
  "app.goToSettings": "Go to Settings to download",
};
