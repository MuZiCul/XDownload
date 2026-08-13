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
- **分阶段进度** — 视频 / 音频流分离下载，显示「视频 / 音频」阶段，ffmpeg 合并时显示循环进度条
- **队列控制** — 任务置顶 / 上移排序，单任务或全部暂停 / 恢复（合并暂停/开始按钮），未完成任务断点续传（保留部分文件）
- **速度控制** — 每任务下载限速；可配置 HLS 分片并发数与重试次数，大幅加速 X 的音视频分离流下载
- **多媒体推文** — 一条推文中的多个视频 / 图片全部下载
- **原子化下载** — 只有（合并）成功后才在下载目录出现文件；中断 / 取消不残留半成品
- **下载历史** — 封面 / 作者 / 时长 / 播放 / 点赞 / 下载时间；即时搜索过滤；时间分级徽标（最近5分钟 → 十年前）；删除时确认是否同时删除磁盘文件；支持重新下载 / 打开位置 / 清空
- **应用内更新** — 在应用内直接下载并安装新版本（直连优先，代理兜底）
- **系统通知** — 下载完成 / 失败时发送系统通知，最小化到托盘也能收到
- **代理支持** — HTTP / SOCKS5，自动检测系统代理
- **Cookies** — 从浏览器导入，访问受限制内容
- **内置工具** — 设置页一键下载 yt-dlp + ffmpeg
- **更新提醒** — 启动时检查 XDownload / yt-dlp / ffmpeg 新版本
- **书签同步** — 一键手动同步 X 书签：预览弹窗列出所有含视频书签（区分已下载/未下载底色），勾选批量或单个入队，已下载的也可重新下载；以「下载历史」为游标，跳过/删除的任务下次同步会重新提示
- **书签目录** — 历次同步的书签（含无视频）持久化到本地 SQLite，可离线浏览、单独下载 / 重新下载
- **浏览器扩展** — MV3 配套扩展在 X 推文上直接添加下载按钮，一键通过深链把视频送入桌面端；v1.3.0 支持自动捕获并推送最新 queryId
- **隐私模式** — 标题以 `***` 遮挡、封面毛玻璃覆盖；设置页 / 状态栏 / 托盘一键开关，重启后保持
- **日志查看器** — 设置页「软件日志」改为在浏览器中打开实时日志查看器（深色主题、级别着色、按日期切换、2 秒自动刷新）
- **深链批量合并** — 扩展连续点击多个视频时自动合并为一批（去重、并发获取信息），只提示一次
- **断点续传开关** — 多任务设置新增开关（默认关），开启后下载中断/失败自动从断点继续，任务面板隐藏暂停/开始按钮
- **来源徽标** — 下载任务 / 历史卡片显示来源徽标（书签 / 批量 / 单链），一眼识别任务从哪个入口加入
- **数据库优化** — 历史与书签改用连接池 + WAL 模式：读取并发、不阻塞下载写入
- **多语言** — English / 简体中文，切换即时生效

## 安装

从 [Releases](https://github.com/MuZiCul/XDownload/releases) 下载最新安装包。

- **Windows**: `XDownload_2.9.1_x64-setup.exe` (NSIS) / `.msi`

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
