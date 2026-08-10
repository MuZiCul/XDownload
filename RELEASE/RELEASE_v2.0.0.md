# XDownload v2.0.0 Release Notes

> 🎉 XDownload 迎来 Rust + Tauri v2 全面重写，从命令行工具进化为跨平台桌面应用。

---

## 🔄 架构重写

- **Java → Rust** — 后端完全使用 Rust 重写，性能大幅提升，内存占用更低，启动秒开
- **CLI → GUI** — 基于 [Tauri v2](https://v2.tauri.app/) 构建原生桌面应用，React 18 + TypeScript + TailwindCSS 打造现代化界面
- **跨平台** — 支持 Windows (NSIS/MSI)、macOS (.dmg)、Linux (.deb/.AppImage) 打包

---

## ✨ 新功能

### 🎨 桌面 GUI
- **多标签界面** — 下载 / 设置 / 关于三个标签页，底部状态栏实时显示 yt-dlp、ffmpeg 版本及代理状态
- **视频解析** — 输入 URL 自动获取视频信息（标题、时长、封面、可用格式列表）
- **多格式下载** — 智能最佳 / 最高画质 / 纯音频 / 自定义格式 ID
- **实时进度** — 下载进度百分比、速度、剩余时间，Toast 通知实时推送

### 🌐 代理支持
- HTTP / SOCKS5 代理配置
- 系统代理自动检测（Windows WinHTTP）
- 代理连通性测试（x.com / Google / GitHub）

### 🍪 Cookies 管理
- 主流浏览器 Cookies 导入（Chrome / Edge / Firefox）
- x.com 登录态自动检测
- 一键验证 Cookies 是否有效

### 🔧 工具管理（设置页）
- **一键下载 yt-dlp** — 自动从 GitHub Releases 拉取最新版
- **一键下载 ffmpeg** — 自动从 gyan.dev 拉取最新 Windows 构建
- 网络检测 + 流式下载进度条 + ffmpeg 解压进度
- 启动时自动检测已安装工具的版本

### 🔔 启动更新检测
- 启动时并行检测 XDownload / yt-dlp / ffmpeg 新版本（GitHub Releases API）
- 毛玻璃弹窗提示，点击跳转设置页下载

### ⚙️ 配置管理
- 下载目录、重试次数、超时等可配置
- 配置导入 / 导出（JSON）
- 界面语言切换（国际化支持）

### 🚀 一键启动
- `pnpm dev` 一键启动开发模式，自动安装依赖、启动前端和后端

---

## 🛠 技术栈

| 层 | v1.x (旧) | v2.0 (新) |
|---|----------|----------|
| 语言 | Java | **Rust** |
| 框架 | 纯 CLI | **Tauri v2** |
| 前端 | 无 | **React 18 + TypeScript** |
| UI | 控制台输出 | **TailwindCSS + Radix UI** |
| 构建 | javac | **Vite** |
| 打包 | 无 | **NSIS / MSI / DMG / AppImage** |
| 图标 | — | **Lucide React** |
| 通知 | — | **Sonner** |
| 状态管理 | — | **TanStack React Query** |

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.0.0) 下载对应平台安装包：

- **Windows**: `XDownload_2.0.0_x64-setup.exe` (NSIS) / `.msi`
- **macOS**: `XDownload_2.0.0_universal.dmg`
- **Linux**: `xdownload_2.0.0_amd64.deb` / `.AppImage`

### 从源码构建

```bash
# 环境要求：Rust (MSVC)、Node.js 18+、pnpm

# 开发模式
pnpm dev

# 打包构建
.\build.bat          # Windows
```

---

## ⚠️ 破坏性变更

- **不再提供 Java CLI** — v2.0 是基于 Rust + Tauri 的桌面 GUI 应用，旧的 Java 命令行入口已移除
- **配置文件格式变更** — 配置从 Java properties 改为 JSON 格式，旧配置需要手动迁移
- **最低系统要求** — Windows 10+、macOS 11+、Linux (glibc 2.31+)

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
