# XDownload

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="XDownload logo" />
</p>

<h1 align="center">XDownload</h1>

<p align="center"><strong>X/Twitter video downloader that just works.</strong></p>

<p align="center">
  🇬🇧 English · <a href=".github/readme/README.zh-CN.md">🇨🇳 简体中文</a>
</p>

<p align="center">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-edition%202021-orange.svg" />
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-lightgrey.svg" />
</p>

---

## What it does

Paste an X/Twitter video link → get the video info → download it. That's it.

Powered by [yt-dlp](https://github.com/yt-dlp/yt-dlp) + ffmpeg, wrapped in a clean desktop app built with **Rust + Tauri v2**.

## Features

- **Paste & parse** — paste a URL, instantly see title / duration / thumbnail / formats
- **Smart best quality** — always downloads the highest quality video + audio (merged automatically), no format picking needed
- **Global progress** — live progress bar in the status bar with speed & ETA, visible on every tab; state survives tab switches
- **Staged progress** — separated video / audio streams show "video" then "audio" stages, with an indeterminate running bar while ffmpeg merges
- **Queue control** — reorder tasks (pin to top / move up), pause & resume a single task or all at once (combined pause/start button), and unfinished downloads resume where they left off (partial files are kept)
- **Speed control** — per-task download rate limit, plus configurable HLS fragment concurrency & retries to dramatically speed up X's fragmented audio/video streams
- **Multi-media tweets** — downloads every video / image in a multi-media tweet
- **Atomic downloads** — files only appear in your download folder after a successful (merged) finish; interrupted / cancelled downloads leave no partial files behind
- **Download history** — cover, author, duration, views, likes, download time; instant search & filter; time-based badges (last 5 minutes → 10 years); delete with confirmation (optionally remove the file from disk too); re-download (re-parses the link), open file location, clear records
- **In-app updates** — download and install new versions right from the app (direct first, proxy fallback)
- **System notifications** — get notified when a download finishes or fails, even when minimized to tray
- **Proxy support** — HTTP or SOCKS5, with automatic system proxy detection
- **Cookies** — import from your browser to access restricted content
- **Built-in tools** — download yt-dlp + ffmpeg right from the settings page
- **Update alerts** — startup checks for new versions of XDownload / yt-dlp / ffmpeg
- **Bookmarks sync** — manual one-click sync of your X bookmarks with a preview dialog: every video bookmark (downloaded or not, color-coded) with per-item checkboxes for batch or single enqueue; already-downloaded ones can be re-downloaded. The download history acts as the cursor, so skipped/deleted tasks show up again on the next sync
- **Bookmark catalogue** — every synced bookmark (video and non-video) is persisted locally in SQLite; browse them offline in a modal and download / re-download any of them
- **Browser extension** — an MV3 companion extension adds a download button right on X posts; one click deep-links the video into the desktop app; v1.3.0 auto-captures and pushes the latest queryId for bookmarks sync
- **Privacy mode** — mask video titles with `***` and blur covers with a frosted-glass overlay; toggle from settings, the status bar, or the tray, and it persists across restarts
- **Log viewer** — open a live, auto-refreshing log viewer (dark theme, level coloring, per-date tabs) in the browser right from the settings page
- **Batch deep-links** — rapid-fire clicks on several X posts get merged into one batch (deduped, concurrent info fetch) and show a single toast instead of N
- **Resume switch** — optional toggle that auto-resumes interrupted downloads from the breakpoint and hides the pause/start buttons (off by default)
- **Source badges** — download task & history cards show where each item came from (bookmark / batch / single link), so you can tell at a glance which entry added it
- **Faster database** — download history & bookmarks use a connection pool + WAL mode: reads run concurrently and never block the download writer
- **Clean cache** — abandoned partial downloads (untouched for over 7 days) are swept at startup instead of wiping everything on arbitrary dates, so interrupted downloads stay resumable
- **Split history view** — the active downloads and the history list each scroll independently (30% / 70% split); the history list is virtualized, staying smooth even with thousands of records
- **i18n** — English & 简体中文, switch instantly without restart

## Install

Grab the latest installer from [Releases](https://github.com/MuZiCul/XDownload/releases).

- **Windows**: `XDownload_2.9.3_x64-setup.exe` (NSIS) / `.msi`

First launch downloads yt-dlp + ffmpeg automatically — or drop them into `bin/` yourself.

## Build from source

Requirements: Rust (MSVC toolchain), Node.js 18+, pnpm.

```bash
pnpm dev          # development (hot reload)
.\build.bat       # package (Windows)
```

## License

[MIT](LICENSE) © MuZiCul

Star ⭐ if this tool helped you solve a little problem.
