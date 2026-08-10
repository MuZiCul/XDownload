# XDownload v2.5.1 Release Notes

> ✨ v2.5.1 聚焦后台常驻与体验细节：新增系统托盘（关闭最小化到托盘 + 右键菜单）、打开文件位置、检查更新弹窗，并优化多项交互细节。

---

## ✨ 新功能

### 🖥 系统托盘
- **托盘图标** — 应用常驻系统托盘，悬浮提示「XDownload」
- **左键单击** — 显示并聚焦主窗口
- **右键菜单** — 显示主窗口 / 打开下载目录 / 退出
- **关闭最小化到托盘** — 点击窗口关闭按钮改为隐藏到托盘，应用后台继续运行；真正退出需通过托盘菜单或应用内退出
- 「退出」自动清理 yt-dlp / ffmpeg 子进程，不残留下载任务

### 📂 打开文件位置
- 已下载的视频旁新增「打开文件位置」按钮
- Windows 下通过 `explorer /select` **精确定位并高亮**目标文件（而非仅打开目录）
- 文件已被删除或无可查记录时自动回退打开下载目录

### 📋 检查更新结果弹窗
- Tools 页「检查更新」完成后弹出**结果弹窗**（替代右上角 toast）
- 展示 yt-dlp / ffmpeg 的当前版本与最新版本
- 未安装 →「下载」按钮；有更新 →「更新」按钮；已最新 → 绿色「✓ 已是最新」
- 检查失败时显示具体错误信息

### 🚫 yt-dlp 错误友好提示
- 将 yt-dlp 常见错误自动翻译为友好中文提示（获取与下载两个环节均生效）：

| 触发关键词 | 友好提示 |
|---|---|
| `Suspended` | 该视频作者已被 X 封禁，无法获取视频内容 |
| `protected` / `not authorized` / `private account` | 该账号为私密/受保护账号，需登录并关注后才能查看 |
| `tweet is unavailable` / `no longer available` | 该推文已被删除或不可用 |
| `nsfw` / `age-restricted` / `requires authentication` | 该内容需要登录或年龄验证后才能查看 |
| `no video could be found` / `is not a video` | 该推文中没有可下载的视频 |
| `guest mode` / `guest token` | 获取访客身份失败，请尝试设置 Cookies 或代理后重试 |
| `rate-limit` / `HTTP Error 429` | 请求过于频繁，请稍后重试 |
| `geoblocked` / `not available in your country` | 该内容在您所在地区不可用 |
| `broadcast no longer exists` | 该直播已结束或不存在 |
| `space not found` / `space ended` | 该 Space 不存在或已结束 |
| `error(s) while querying api` | X 接口返回异常，请稍后重试 |

- 未命中的错误保持原文，便于排查问题

---

## 🐛 修复与优化

- **代理重复检测** — 设置页切页时代理状态不再每次重新测试；代理配置未变时复用上次结果，不闪烁「测试中...」
- **打开文件位置打开"此电脑"** — 根因：`cmd /C` 二次解析引号导致路径被拆分；改为直接调用 `explorer /select` 并交由 Rust 处理参数转义
- **移除重试次数配置** — 删除下载选项中的「重试」输入框及其后端传参，yt-dlp 使用内置默认重试（10 次），下载可靠性不受影响

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

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.5.1) 下载对应平台安装包：

- **Windows**: `XDownload_2.5.1_x64-setup.exe` (NSIS) / `.msi`
- **macOS**: `XDownload_2.5.1_universal.dmg`
- **Linux**: `xdownload_2.5.1_amd64.deb` / `.AppImage`

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

- 从 v2.5.0 升级**无需迁移配置**
- 托盘功能仅 Windows 系统启用；关闭窗口后如需完全退出，请使用托盘菜单「退出」

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
