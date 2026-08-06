<p align="center">
  <img src="../../src-tauri/icons/icon.png" width="128" height="128" alt="XDownload logo" />
</p>

<h1 align="center">XDownload</h1>

<p align="center"><strong>开箱即用的 X/Twitter 视频下载器。</strong></p>

<p align="center">
  <a href="../../README.md">🇬🇧 English</a> · 🇨🇳 简体中文
</p>

<p align="center">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-edition%202021-orange.svg" />
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" />
</p>

---

## 它能做什么

粘贴 X/Twitter 视频链接 → 获取视频信息 → 下载。就这么简单。

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) + ffmpeg，使用 **Rust + Tauri v2** 构建的轻量桌面应用。

## 功能特性

- **粘贴即解析** — 输入 URL，立刻显示标题 / 时长 / 封面 / 可用格式
- **智能格式** — 最佳画质、视频+音频、仅音频、自定义格式 ID
- **实时进度** — 可见进度条（百分比 / 速度 / 剩余时间），含合并与后处理阶段
- **下载历史** — 记录已下载视频与时间，一键重新下载
- **代理支持** — HTTP / SOCKS5，自动检测系统代理
- **Cookies** — 从浏览器导入，访问受限制内容
- **内置工具** — 设置页一键下载 yt-dlp + ffmpeg
- **更新提醒** — 启动时检查 XDownload / yt-dlp / ffmpeg 新版本

## 安装

从 [Releases](https://github.com/MuZiCul/XDownload/releases) 下载最新安装包。

- **Windows**: `XDownload_2.5.0_x64-setup.exe` (NSIS) / `.msi`
- **macOS**: `.dmg`
- **Linux**: `.deb` / `.AppImage`

首次启动会自动下载 yt-dlp + ffmpeg，也可以手动放入 `bin/` 目录。

## 从源码构建

环境要求：Rust (MSVC toolchain)、Node.js 18+、pnpm。

```bash
pnpm dev          # 开发模式（热重载）
.\build.bat       # 打包构建（Windows）
```

## 许可证

[MIT](../../LICENSE) © MuZiCul

如果这个工具帮你解决了一点问题，请给它点个 ⭐
