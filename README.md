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
- **Multi-media tweets** — downloads every video / image in a multi-media tweet
- **Atomic downloads** — files only appear in your download folder after a successful (merged) finish; interrupted / cancelled downloads leave no partial files behind
- **Download history** — review what you downloaded and when: cover, author, duration, views, likes, download time; re-download (re-parses the link), open file location, delete / clear records
- **In-app updates** — download and install new versions right from the app (direct first, proxy fallback)
- **System notifications** — get notified when a download finishes or fails, even when minimized to tray
- **Proxy support** — HTTP or SOCKS5, with automatic system proxy detection
- **Cookies** — import from your browser to access restricted content
- **Built-in tools** — download yt-dlp + ffmpeg right from the settings page
- **Update alerts** — startup checks for new versions of XDownload / yt-dlp / ffmpeg
- **i18n** — English & 简体中文, switch instantly without restart

## Install

Grab the latest installer from [Releases](https://github.com/MuZiCul/XDownload/releases).

- **Windows**: `XDownload_2.7.1_x64-setup.exe` (NSIS) / `.msi`

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
