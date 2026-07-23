# XDownload v1.0.5

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的桌面视频下载器，Swing GUI，开箱即用。

## 特性

- **便携免安装** — 单目录 exe，内置 JRE，复制即用
- **现代 GUI** — FlatLaf 扁平风格，中英双语自适应
- **国内外自动感知** — 启动检测网络环境，海外跳过代理，国内引导配置
- **浏览器 Cookies 直读** — 自动扫描 Chrome / Firefox / Edge / Brave / Opera
- **智能回退** — 当前浏览器不可用时自动切换
- **配置持久化** — 代理、Cookies、输出目录、语言偏好自动保存
- **工具管理** — yt-dlp / ffmpeg 一键下载更新
- **日志面板** — yt-dlp 输出实时捕获，方便排查

## 快速开始

### 便携版（推荐）

1. 双击 `build.bat` 构建
2. 将 `build\XDownload\` 复制到任意位置
3. 双击 `XDownload.exe`

### Java 运行

```bash
javac -encoding UTF-8 -cp lib/flatlaf-3.5.jar -d out/classes -sourcepath src src/Main.java
java -cp "lib/flatlaf-3.5.jar;out/classes" Main
```

## 界面

```
┌──────────────────────────────────────────────────┐
│  [Download]  [Settings]  [About]                  │
├──────────────────────┬───────────────────────────┤
│ URL: [__________]    │ ┌─ 状态信息 ──────────────┐│
│ [Fetch Info] [Paste] │ │ yt-dlp: 2026.07.04      ││
├──────────────────────┤ │ Proxy: none              ││
│ Video Info           │ │ Cookies: chrome          ││
│ Title / Author       │ │ ffmpeg: OK               ││
├──────────────────────┤ │ ──────────────────────── ││
│ 格式列表              │ │ [最佳] [视频+音频] [仅音频]││
│ ID │Ext│Resolution   │ │ 重试次数: [5] [下载]     ││
│ 18 │mp4│640x360      │ │ 45.2% | 2.5MiB/s        ││
└──────────────────────┴───────────────────────────┘
```

## 设置

| 设置项 | 说明 |
|--------|------|
| 视频保存位置 | 选择下载目录，自动持久化 |
| 代理 | 无代理 / 手动代理 + 测试 / 自动检测国内外 |
| Cookies | 选择浏览器 + 验证 + 保存 |
| Tools | yt-dlp / ffmpeg 下载和更新 |
| 语言 | 中文 / English，重启生效 |
| View Log | 系统默认程序打开日志文件 |

## 构建便携版

```bash
build.bat
```

过程：`javac` → `jar` → `jlink` (java.base+logging+crypto.ec+desktop) → `jpackage` (app-image)

输出 `build\XDownload\`：

```
XDownload\
├── XDownload.exe          ← 双击启动
├── app\                   ← 主程序 JAR
├── runtime\               ← 内置 JRE (~50MB)
├── bin\                   ← yt-dlp.exe / ffmpeg.exe
├── config\                ← settings.json + 日志
└── downloads\             ← 下载输出
```

## 项目结构

```
XDownload/
├── src/
│   ├── Main.java                      # 入口
│   ├── ui/gui/
│   │   ├── XDownloadApp.java          # GUI 启动 (FlatLaf + 日志)
│   │   ├── MainFrame.java             # 主窗口 (标签页 + 状态栏)
│   │   ├── StartupWizard.java         # 首次运行引导
│   │   ├── panels/                    # 面板 (Download/Settings/About/Log)
│   │   └── workers/                   # SwingWorker 子类
│   ├── downloader/
│   │   └── YtDlpDownloader.java       # yt-dlp 封装
│   ├── model/
│   │   ├── VideoInfo.java             # 视频信息 + 格式
│   │   └── DownloadConfig.java        # 下载配置
│   └── util/
│       ├── AppHome.java               # 路径解析
│       ├── Bootstrap.java             # 依赖下载
│       ├── ProcessHelper.java         # 进程调用 + Cookies 验证
│       ├── ProxyConfig.java           # 代理管理
│       ├── ConfigManager.java         # 配置持久化
│       ├── ChromeCookies.java         # Chrome DB 备份
│       ├── NetworkDetect.java         # 国内外检测
│       ├── I18n.java                  # 中英双语
│       └── Version.java               # 版本号
├── lib/flatlaf-3.5.jar                # FlatLaf 皮肤
├── build.bat                          # 构建脚本
└── README.md
```

## 常见问题

### 下载 X/Twitter 视频提示需要登录

程序自动从浏览器提取 Cookies。Chrome 锁库时自动回退到 Firefox → Edge → Brave。

### 国内无法访问外网

启动时自动检测，国内环境引导输入代理地址。

### exe TLS 握手失败

jlink 已包含 `jdk.crypto.ec` 模块，支持 ECDHE 加密套件。

### 下载慢 / GitHub 无法访问

设置中先配置代理，再使用 Tools 下载 yt-dlp / ffmpeg。

## 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [FFmpeg](https://ffmpeg.org)
- [FlatLaf](https://www.formdev.com/flatlaf/)

## License

MIT License — 详见 [LICENSE](https://github.com/MuZiCul/XDownload)

Copyright (c) 2025 MuZiCul
