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
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-lightgrey.svg" />
</p>

---

## 它能做什么

粘贴 X/Twitter 视频链接 → 获取视频信息 → 下载。就这么简单。

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) + ffmpeg，使用 **Rust + Tauri v2** 构建的轻量桌面应用。

## 功能特性

- **粘贴即解析** — 输入 URL，立刻显示标题 / 时长 / 封面 / 格式
- **智能最佳画质** — 自动下载最高画质视频 + 音频（自动合并），无需手动选格式
- **全局进度** — 状态栏实时进度条（百分比 / 速度 / 剩余时间），任意 Tab 可见；切换页面状态不丢失
- **多媒体推文** — 一条推文中的多个视频 / 图片全部下载
- **下载历史** — 查看下载记录与时间，支持打开 / 删除 / 清空
- **应用内更新** — 在应用内直接下载并安装新版本（直连优先，代理兜底）
- **系统通知** — 下载完成 / 失败时发送系统通知，最小化到托盘也能收到
- **代理支持** — HTTP / SOCKS5，自动检测系统代理
- **Cookies** — 从浏览器导入，访问受限制内容
- **内置工具** — 设置页一键下载 yt-dlp + ffmpeg
- **更新提醒** — 启动时检查 XDownload / yt-dlp / ffmpeg 新版本
- **多语言** — English / 简体中文，切换即时生效

## 安装

从 [Releases](https://github.com/MuZiCul/XDownload/releases) 下载最新安装包。

- **Windows**: `XDownload_2.5.2_x64-setup.exe` (NSIS) / `.msi`

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
