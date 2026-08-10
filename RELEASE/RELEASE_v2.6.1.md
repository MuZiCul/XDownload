# XDownload v2.6.1 Release Notes

> ✨ v2.6.1 聚焦界面细节与稳定性：日志目录独立管理、设置页新增「软件日志」入口、禁用态视觉统一、toast 样式优化、取消下载提示国际化。

---

## ✨ 新功能

- **设置页「软件日志」按钮** — 在「退出」按钮左侧新增入口，点击直接打开 `logs/` 日志目录
- **日志目录迁移** — 日志从 `config/` 独立到根目录 `logs/` 文件夹，便于管理与 git 忽略

## 🐛 修复与优化

- **取消下载提示** — 用户主动取消时显示「下载失败: 用户主动取消」（可识别），且文案国际化（中/英）
- **禁用态视觉统一** — 禁用按钮/输入框统一显示禁止光标 + 40% 透明度，代理设置中类型/主机/端口在非手动模式下明显置灰
- **toast 样式** — 中性 toast 改浅灰蓝底 + 边框，与软件背景区分；所有 toast 统一描边
- **下载按钮条件渲染** — 未获取到视频信息时不显示下载按钮；有信息且本地无文件时只显示「开始下载」
- **系统通知/下载提示国际化** — 下载完成/失败的通知与 toast 全部跟随界面语言

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.6.1) 下载对应平台安装包：

- **Windows**: `XDownload_2.6.1_x64-setup.exe` (NSIS) / `.msi`

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

- 从 v2.6.0 升级**无需迁移配置**
- 日志目录自动迁移：新日志写入 `logs/`，旧的 `config/xdownload.log.*` 不再更新（可手动删除）
- `logs/` 目录已加入 `.gitignore`，不会被提交

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
