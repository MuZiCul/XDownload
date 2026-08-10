# XDownload v2.5.2 Release Notes

> ✨ v2.5.2 聚焦下载体验、国际化与工程健壮性：下载状态全局化、智能最佳画质、多媒体推文全部下载、应用内更新、系统通知、下载历史页、前端国际化，并修复多项可靠性问题。

---

## ✨ 新功能

### 📊 下载状态全局化
- 底部状态栏常驻**实时进度条**（渐变 + 流光动画、速度/剩余时间、取消按钮），任意 Tab 都可见
- 下载页切换 Tab 后解析结果、格式选择、下载状态**全部保留**（不再丢失）
- 下载完成/失败：状态栏提示 + 系统通知（最小化到托盘也能收到）

### 🎬 智能最佳画质
- 默认格式改为 `bestvideo+bestaudio/best`：优先合并**最高画质视频流 + 音频流**（不再被 `-f best` 静默降级到低分辨率单文件）
- 自动为 yt-dlp 注入 `--ffmpeg-location` 指向内置 ffmpeg，确保合并真正可用
- bin 内无 ffmpeg 时下载前弹窗提醒

### 📹 多媒体推文全部下载
- 多视频/多图推文解析全部媒体条目并**全部下载**（yt-dlp 自动序号命名），不再只取第一个
- 视频信息栏展示「该推文包含 N 条媒体，将全部下载」

### ⬇️ 应用内更新
- 检测到新版本后可在应用内**直接下载安装包**（先直连、失败自动走代理）并**静默安装**（NSIS /S / MSI /qn）
- 更新检查增加 GitHub API 403 限流的**网页回退**，不再被限流卡死

### 🔔 系统通知
- 接入 `tauri-plugin-notification`：下载完成/失败发送 Windows 系统通知

### 🗂 下载历史页
- 新增「历史」Tab：展示下载标题与时间，支持**打开文件位置 / 删除单条 / 清空全部**
- 历史记录记录真实视频标题（此前为 id）

### 🌐 前端国际化
- 自研轻量 i18n（零依赖）：**中/英即时切换**，无需重启
- 覆盖全部界面文案（约 170 条）：Tab、下载页、设置页、历史页、关于页、更新弹窗、toast、错误提示
- 免责声明、cookies 验证提示、错误映射全部随语言切换

### 🕐 日志本地时区
- 日志文件按**本地日期**轮转、时间戳显示本地时区（此前为 UTC，国内差一天/8 小时）

---

## 🐛 修复与优化

- **并发下载互斥锁** — 后端原子标志拒绝重复下载，杜绝双 yt-dlp 进程
- **SOCKS5 代理 scheme 被忽略** — reqwest 路径统一使用配置的 scheme（此前硬编码 http://）
- **重新下载丢弃新文件** — 文件名冲突时保留新文件并生成序号（`标题 (1).mp4`），不再"记录成功却是旧文件"
- **更新检查/网络探测不走代理** — 全部挂配置代理；工具下载**先直连（8s 快速失败）后代理**
- **代理测试行为** — 测试与保存分离：测试不再改运行时代理、不再测试成功即落盘
- **`twitter.com` 链接被误拒** — 域名白名单校验（支持 x.com / twitter.com 及子域名）
- **进度条 0→100 直跳** — 进度解析移到 stderr + `--progress-template` 强制逐行输出
- **下载目录统一绝对路径** — 文件不再落到 `src-tauri\downloads`，始终在配置的下载目录
- **CSP 安全配置** — 从 `null` 改为明确的白名单指令
- **`download-complete` / `download-error` 事件** — 补齐后端异常分支事件，前端全局监听
- **剪贴板静默失败** — 接入 `tauri-plugin-clipboard-manager`，失败有明确提示
- **下载历史标题** — `DownloadConfig` 携带标题写入历史

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
| 新增插件 | clipboard-manager / notification |

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.5.2) 下载对应平台安装包：

- **Windows**: `XDownload_2.5.2_x64-setup.exe` (NSIS) / `.msi`
- **macOS**: `XDownload_2.5.2_universal.dmg`
- **Linux**: `xdownload_2.5.2_amd64.deb` / `.AppImage`

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

- 从 v2.5.1 升级**无需迁移配置**
- 下载格式策略调整为「智能最佳」（自动合并最高画质），无需手动选择格式
- 首次使用应用内更新：需在 GitHub Release 上传安装包资产；系统通知首次会请求权限

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
