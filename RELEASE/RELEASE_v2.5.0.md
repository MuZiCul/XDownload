# XDownload v2.5.0 Release Notes

> ✨ v2.5 聚焦下载体验优化：新增下载进度条、下载历史与重复下载检测、SOCKS5 代理支持，并修复多项核心问题。

---

## ✨ 新功能

### 📊 下载进度条
- **可见进度条** — 下载选项卡片内新增实时进度条，显示百分比、速度、剩余时间
- **阶段感知** — 自动识别下载/合并音视频流/后处理阶段，合并时显示「正在合并音视频流...」，不再卡在 100%

### 🗂 下载历史与重复下载检测
- **下载历史** — 记录已下载视频的 id、保存路径、下载时间（`config/downloads.json`）
- **已下载标记** — 解析视频后自动检测，信息卡显示「✓ 已下载 · 时间」
- **重复下载确认** — 已下载视频再次点击下载时弹窗确认，可一键「重新下载」
- **智能防误判** — 每次下载前实时检查文件是否仍存在；若文件已被删除则直接重新下载，不弹确认

### 🌐 SOCKS5 代理支持
- 代理类型支持 **HTTP / SOCKS5** 协议选择，并持久化到配置
- 代理测试、系统代理检测均支持新协议
- 修复代理参数拼接缺陷（不再写死 http://）

### 📁 下载目录快捷打开
- 设置页「视频保存位置」新增「打开」按钮，一键在文件管理器中打开下载目录

### 🔧 工具管理增强
- Tools 栏工具状态三态化：`Download`（未安装）/ `Update`（有更新）/ `Latest`（最新）
- 新增「检查更新」按钮，手动刷新工具版本状态
- 底部状态栏圆点三态指示：有更新=🟡黄 / 可用=🟢绿 / 未安装=🔴红

### 📛 文件名清理
- 下载文件名自动过滤，仅保留中英文、数字、`-`、`#`、`+`（空格、标点、emoji 等自动移除），扩展名保留

---

## 🐛 修复

- **下载进度始终为 0** — 根因：yt-dlp 进度输出在 stdout 而非 stderr，已改为从 stdout 解析
- **GBK 管道错误** `[Errno 22] Invalid argument` — 强制 yt-dlp 以 UTF-8 输出（`--encoding utf-8` + `PYTHONIOENCODING`）
- **合并阶段进度不可见** — 识别 `[Merger]` / `[ExtractAudio]` 等后处理阶段行
- **取消下载不生效** — 现在取消会真正终止 yt-dlp / ffmpeg 进程树
- **设置页配置不同步** — 保存代理/Cookies/目录后下载页立即生效
- **多视频帖子解析不稳** — `--dump-json` 多行输出逐行解析，取第一个有效 JSON
- **重复下载弹窗按钮换行** — 图标与文字同行显示
- **启动脚本中文乱码报错** — `start-dev.bat` 改为纯 ASCII

---

## 🛠 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | [Tauri v2](https://v2.tauri.app/) |
| 后端 | Rust (edition 2021) |
| 前端 | React 18 + TypeScript |
| 构建工具 | Vite |
| UI 框架 | TailwindCSS + Radix UI |
| 图标 | Lucide React |
| 通知 | Sonner |
| 状态管理 | TanStack React Query |
| 下载引擎 | yt-dlp + ffmpeg |

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.5.0) 下载对应平台安装包：

- **Windows**: `XDownload_2.5.0_x64-setup.exe` (NSIS) / `.msi`
- **macOS**: `XDownload_2.5.0_universal.dmg`
- **Linux**: `xdownload_2.5.0_amd64.deb` / `.AppImage`

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

- 从 v2.0 升级**无需迁移配置**，配置文件格式保持不变
- 首次使用「已下载」检测功能时，仅对本次升级后下载的视频生效（历史下载不会回溯标记）
- SOCKS5 代理：请确保本地代理工具支持 SOCKS5 协议（如 clash / v2ray 等）

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
