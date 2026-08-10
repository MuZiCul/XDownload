# XDownload v2.7.1 Release Notes

> 🛠 v2.7.1 为修复版本，聚焦更新安装流程与相关交互问题。

---

## 🐛 修复

- **更新后安装程序未启动** — 安装器增加 `CREATE_BREAKAWAY_FROM_JOB` 标志，使其脱离应用进程的 Job Object，不再随应用退出被终止；并在应用退出前延迟启动安装器，避免文件占用导致静默安装失败
- **关于页「检查更新 → 前往下载」** — 点击 toast 中的「前往下载」改为弹出与启动检查一致的拟态玻璃更新窗（支持直接下载更新 / 前往 GitHub），不再直接跳转浏览器

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.7.1) 下载对应平台安装包：

- **Windows**: `XDownload_2.7.1_x64-setup.exe` (NSIS) / `.msi`

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

- 从 v2.7.0 升级**无需迁移配置**，直接覆盖安装即可
- 安装前请确保应用已完全退出（更新流程会自动退出）

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
