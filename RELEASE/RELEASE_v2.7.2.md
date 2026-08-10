# XDownload v2.7.2 Release Notes

> 🛠 v2.7.2 修复浏览器下载扩展的协议触发问题，并消除编译警告。

---

## 🐛 修复

- **浏览器扩展点击「下载」无响应**
  - **协议未注册**：`tauri-plugin-deep-link` 不会自动注册自定义协议，现于应用启动时显式调用 `register_all()` 注册 `xdownload://`
  - **缺少 single-instance 插件**：Windows 深链通过「协议拉起新实例」实现，新增 `tauri-plugin-single-instance`（启用 `deep-link` feature）将深链 URL 转发给已运行的主实例，并自动恢复/聚焦主窗口
- **编译警告清零**：清理 26 个 dead_code 误报与 2 个 unused import，打包输出更干净

## ✨ 其他

- 新增 `scripts/register-protocol.ps1`：手动注册/反注册 `xdownload://` 协议（应急场景）

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.7.2) 下载对应平台安装包：

- **Windows**: `XDownload_2.7.2_x64-setup.exe` (NSIS) / `.msi`

### 从源码构建

```bash
# 环境要求：Rust (MSVC)、Node.js 18+、pnpm

# 开发模式
pnpm dev

# 打包构建
.\build.bat          # Windows
```

---

## ⚠️ 升级说明

- 从 v2.7.1 升级**无需迁移配置**，直接覆盖安装即可
- 安装完成后首次启动会自动注册 `xdownload://` 协议

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
