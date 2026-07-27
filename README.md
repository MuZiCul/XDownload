# XDownload v1.1

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的 X/Twitter 视频下载器，Swing GUI，开箱即用。

## 特性

- **便携免安装** — 单目录 exe，内置 JRE，复制即用
- **现代 GUI** — FlatLaf 扁平风格，中英双语自适应
- **X/Twitter 专精** — 仅允许 x.com 链接，获取前预检网络可达性，5 秒内反馈结果
- **Windows 系统代理自检测** — 启动时自动读取系统代理设置（Clash / v2rayN 等），即时生效
- **国内外自动感知** — 启动检测网络环境，海外跳过代理，国内引导配置
- **组件智能下载** — yt-dlp/ffmpeg 缺失时先检测 GitHub 可达性，不可达引导配置代理
- **浏览器 Cookies 直读** — 自动扫描 Chrome / Firefox / Edge / Brave / Opera
- **配置管理** — 保存配置 / 应用配置（支持自定义配置文件路径），白名单校验
- **配置持久化** — 代理、Cookies、输出目录、语言偏好自动保存
- **工具管理** — yt-dlp / ffmpeg 一键下载更新
- **启动优化** — 异步初始化不阻塞主窗口，版本号缓存避免重复进程启动
- **可取消弹窗** — 启动向导关闭即终止所有后台任务

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
├──────────────────────┤ │ Proxy: 127.0.0.1:7897   ││
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
| 代理 | 无代理 / 手动代理 + 测试 / 自动检测国内外；启动时自动读取 Windows 系统代理 |
| Cookies | 选择浏览器 + 验证 + 保存 |
| Tools | yt-dlp / ffmpeg 下载和更新 |
| 语言 | 中文 / English，重启生效 |
| 保存配置 | 将所有设置保存到 `config/settings.json` |
| 应用配置 | 弹窗选择「应用默认配置」或「选择配置文件位置」，支持部分应用和白名单校验 |

## v1.1 新增

| 功能 | 说明 |
|------|------|
| Windows 系统代理自检测 | 启动时自动读取注册表代理设置（Clash/v2rayN 等），无需手动填写 |
| x.com 链接限制 | 仅允许包含 x.com 的 URL，非 x.com 链接立即弹窗拒绝 |
| 网络预检 | 获取视频信息前快速检测 x.com 可达性（5 秒超时），网络不通直接提示 |
| GitHub 连通预检 | 下载 yt-dlp/ffmpeg 前先检测 GitHub 可达性，不可达引导配置代理 |
| 配置管理按钮 | 保存配置 / 应用配置（支持自定义 JSON 文件），浏览器名和语言白名单校验 |
| 启动性能优化 | 异步初始化网络/Cookies/代理检测不阻塞主窗口，yt-dlp 版本号首次缓存 |
| 可取消启动弹窗 | 向导弹窗关闭即终止所有后台 SwingWorker + yt-dlp 进程 |
| 控制台模式移除 | 全面转向 GUI，`ConsoleUI` 已删除 |
| 版本升级 | v1.0.5 → v1.1 |

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
│   │   ├── panels/                    # 面板 (Download/Settings/About/Log/Cookies/Proxy)
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
│       ├── ProxyConfig.java           # 代理管理 + 系统代理检测
│       ├── ConfigManager.java         # 配置持久化
│       ├── ChromeCookies.java         # Chrome DB 备份
│       ├── NetworkDetect.java         # 国内外/GitHub/x.com 检测
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

1. **有 Clash/v2rayN 等工具？** 启动时自动读取 Windows 系统代理，无需手动配置
2. **手动配置：** 启动向导自动检测，国内环境引导输入代理地址

### 下载慢 / GitHub 无法访问

1. 先配置代理（设置 → 代理 → 手动代理 → 测试代理）
2. 再使用 Tools 下载 yt-dlp / ffmpeg（下载前会自动检测 GitHub 可达性）

### exe TLS 握手失败

jlink 已包含 `jdk.crypto.ec` 模块，支持 ECDHE 加密套件。

### 配置文件迁移/备份

使用设置中的「保存配置」导出当前设置，新环境中「应用配置」→「选择配置文件位置」导入。

## 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [FFmpeg](https://ffmpeg.org)
- [FlatLaf](https://www.formdev.com/flatlaf/)

## License

MIT License — 详见 [LICENSE](https://github.com/MuZiCul/XDownload)

Copyright (c) 2025 MuZiCul
