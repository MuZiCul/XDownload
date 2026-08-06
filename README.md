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
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" />
</p>

---

## What it does

Paste an X/Twitter video link → get the video info → download it. That's it.

Powered by [yt-dlp](https://github.com/yt-dlp/yt-dlp) + ffmpeg, wrapped in a clean desktop app built with **Rust + Tauri v2**.

## Features

- **Paste & parse** — paste a URL, instantly see title / duration / thumbnail / formats
- **Smart formats** — best quality, video+audio, audio-only, or custom format ID
- **Live progress** — visible progress bar with speed & ETA, including merge / post-process stages
- **Download history** — know what you downloaded and when; re-download with one click
- **Proxy support** — HTTP or SOCKS5, with automatic system proxy detection
- **Cookies** — import from your browser to access restricted content
- **Built-in tools** — download yt-dlp + ffmpeg right from the settings page
- **Update alerts** — startup checks for new versions of XDownload / yt-dlp / ffmpeg

## Install

Grab the latest installer from [Releases](https://github.com/MuZiCul/XDownload/releases).

- **Windows**: `XDownload_2.5.0_x64-setup.exe` (NSIS) / `.msi`
- **macOS**: `.dmg`
- **Linux**: `.deb` / `.AppImage`

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
