# XDownload v2.5

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的跨平台桌面视频下载器，使用 Rust + Tauri v2 构建。

## ✨ 功能

- **视频解析** — 输入 URL 自动获取视频信息（标题、时长、封面、可用格式列表）
- **多格式下载** — 智能最佳 / 最高画质 / 纯音频 / 自定义格式 ID
- **实时进度** — 下载进度百分比、速度、剩余时间
- **后处理** — 自动调用 ffmpeg 合并音视频流、提取音频、嵌入字幕和封面
- **代理支持** — HTTP / SOCKS5 代理，支持系统代理自动检测
- **Cookies 管理** — 浏览器 Cookies 导入验证，支持 x.com 登录态检测
- **配置管理** — 下载目录、重试次数、超时等可配置，支持配置导入/导出
- **启动更新检测** — 启动时自动检查 XDownload / yt-dlp / ffmpeg 是否有新版本，毛玻璃弹窗提示
- **工具管理** — 设置页一键下载 yt-dlp + ffmpeg，自带网络检测、下载进度条、解压进度
- **多标签界面** — 下载 / 设置 / 关于，底部状态栏显示 yt-dlp、ffmpeg 版本及代理状态

## 🛠 技术栈

| 层 | 技术 |
|---|------|
| 桌面框架 | [Tauri v2](https://v2.tauri.app/) |
| 后端 | Rust |
| 前端 | React 18 + TypeScript |
| 构建工具 | Vite |
| UI 框架 | TailwindCSS + Radix UI |
| 图标 | Lucide React |
| 通知 | Sonner |
| 状态管理 | TanStack React Query |
| 下载引擎 | yt-dlp + ffmpeg |
| 包管理 | pnpm |

## 🚀 快速开始

### 环境要求

- **Rust** (MSVC toolchain, Windows) — [rustup.rs](https://rustup.rs/)
- **Node.js** 18+ + **pnpm** — `npm i -g pnpm`
- **yt-dlp** & **ffmpeg** — 首次启动会自动从官方源下载最新版，也可手动放入 `bin/` 目录

### 一键启动


```bash
# 在根目录下
pnpm dev
```


### 开发模式

```bash
# 安装前端依赖
cd ui && pnpm install

# 启动开发服务器（热重载）
cd ../src-tauri
cargo tauri dev
```

### 打包构建

```bash
# 双击运行或命令行执行
.\build.bat

# 产物位置
#   NSIS 安装包 → src-tauri\target\release\bundle\nsis\XDownload_2.5.0_x64-setup.exe
#   MSI 安装包  → src-tauri\target\release\bundle\msi\
```

## 📁 项目结构

```
XDownload-rust/
├── build.bat                  # 一键打包脚本
├── bin/                       # yt-dlp.exe / ffmpeg.exe 存放目录
├── ui/                        # 前端 (React + Vite)
│   ├── src/
│   │   ├── App.tsx            # 应用入口 + 启动更新检测弹窗
│   │   ├── main.tsx           # ReactDOM 挂载
│   │   ├── lib/
│   │   │   ├── bindings.ts    # Tauri 命令绑定
│   │   │   └── types.ts       # TypeScript 类型定义
│   │   ├── components/
│   │   │   ├── download/      # 下载页：URL栏、视频信息、格式表、下载控制
│   │   │   ├── settings/      # 设置页：目录、代理、Cookies、工具下载
│   │   │   ├── layout/        # 布局：标签栏、状态栏
│   │   │   └── about/         # 关于页 + 手动检测更新
│   │   └── hooks/             # 自定义 hooks
│   ├── package.json
│   └── vite.config.ts
├── src-tauri/                 # 后端 (Rust + Tauri)
│   ├── src/
│   │   ├── main.rs            # Windows 入口
│   │   ├── lib.rs             # Tauri Builder + 命令注册
│   │   ├── commands/          # Tauri 命令层
│   │   │   ├── download.rs    # 获取视频信息 / 开始下载 / 取消下载
│   │   │   ├── settings.rs    # 配置读写
│   │   │   ├── proxy.rs       # 代理测试 / 状态 / 切换
│   │   │   ├── cookies.rs     # Cookies 验证 / 扫描
│   │   │   ├── bootstrap.rs   # 工具检查 / 下载 / 网络检测
│   │   │   └── update.rs      # 版本更新检测 (app / yt-dlp / ffmpeg)
│   │   ├── downloader/        # 下载引擎
│   │   │   ├── ytdlp.rs       # yt-dlp CLI 封装
│   │   │   ├── parser.rs      # JSON 输出解析
│   │   │   └── progress.rs    # 进度行正则匹配
│   │   ├── models/            # 数据模型
│   │   ├── services/          # 业务逻辑层
│   │   │   ├── config.rs      # 配置管理器
│   │   │   ├── proxy.rs       # 代理配置
│   │   │   ├── cookies.rs     # Cookies 管理
│   │   │   ├── bootstrap.rs   # 工具自动下载（动态版本 + 流式进度）
│   │   │   ├── network.rs     # 网络检测（Google / x.com / GitHub）
│   │   │   └── i18n.rs        # 国际化
│   │   └── utils/             # 工具函数
│   ├── Cargo.toml
│   ├── tauri.conf.json        # Tauri 配置
│   └── icons/                 # 应用图标
└── package.json               # 根工作区（Tauri CLI 入口）
```

## ⚙️ 架构

```
┌─────────────────────────────────────────────┐
│                   Frontend                    │
│         React + Vite + TailwindCSS           │
│          TanStack Query (状态)               │
│          Sonner (Toast 通知)                 │
└──────────────┬──────────────────────────────┘
               │ invoke() / event listen
┌──────────────▼──────────────────────────────┐
│                 Tauri Bridge                  │
│   ┌──────────────────────────────────────┐  │
│   │           Commands Layer              │  │
│   │  download / settings / proxy /        │  │
│   │  cookies / bootstrap / update         │  │
│   └──────────┬───────────────────────────┘  │
│   ┌──────────▼───────────────────────────┐  │
│   │          Services Layer               │  │
│   │  ProxyConfig / CookieManager /        │  │
│   │  ConfigManager / Bootstrap /          │  │
│   │  NetworkDetect / I18n                 │  │
│   └──────────┬───────────────────────────┘  │
│   ┌──────────▼───────────────────────────┐  │
│   │         Downloader Layer              │  │
│   │  YtDlpDownloader / DownloadConfig     │  │
│   │  ProgressParser (stderr regex)        │  │
│   └──────────┬───────────────────────────┘  │
│              │ spawn + stdin/stdout/stderr    │
│   ┌──────────▼───────────────────────────┐  │
│   │        yt-dlp + ffmpeg                │  │
│   │  (首次启动自动下载，启动时检测更新)      │  │
│   └──────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

**下载流程**：前端发起请求 → Tauri Command 异步生成 yt-dlp 子进程 → 逐行解析 stderr 进度 → 通过 Tauri Event 实时推送给前端 → Toast 显示进度。

**更新检测**：启动时并行检测 XDownload（GitHub Releases API）、yt-dlp（GitHub Releases API）、ffmpeg（gyan.dev release-version）→ 有新版本则毛玻璃弹窗提醒 → 点击跳转设置页下载。

**工具下载**：点击下载 → 毛玻璃弹窗检测网络（Google HEAD）→ 流式下载实时进度 → ffmpeg 额外显示解压进度 → 完成自动关闭。

## 📄 License

MIT — 详见 [src-tauri/Cargo.toml](src-tauri/Cargo.toml)

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
