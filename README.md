# XDownload — X视频下载工具

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的命令行视频下载器，支持 1000+ 网站，开箱即用。

## 特性

- **零配置启动** — 首次运行自动下载 yt-dlp，无需手动安装依赖
- **浏览器 Cookies 直读** — 从 Chrome/Firefox/Edge 自动提取登录态，下载需登录内容
- **智能回退** — Chrome 锁库时自动切 Firefox → Edge → Brave → Opera
- **代理支持** — 全局代理，启动时自动验证连通性
- **配置持久化** — 代理、Cookies 偏好保存到 `config/settings.json`，下次自动加载
- **纯控制台** — 仅显示进度条，干净简洁
- **交互 + CLI 双模式** — 既可一步步选，也可一行命令搞定

## 依赖

| 组件 | 说明 |
|------|------|
| yt-dlp | **必须**。首次运行自动下载到 `bin/`，17MB |
| ffmpeg | 推荐。部分格式合并/转码需要（启动时可选择下载，80MB） |
| Java 11+ | 运行环境 |

## 快速开始

```bash
# 编译
javac -encoding UTF-8 -d out/production/XDownload -sourcepath src src/Main.java

# 交互模式
java -cp out/production/XDownload -Dfile.encoding=UTF-8 Main

# 命令行模式
java -cp out/production/XDownload -Dfile.encoding=UTF-8 Main <URL>
```

首次启动会依次引导：

```
  ━━━ 环境检查 ━━━
  代理地址 (host:port): 127.0.0.1:7890     ← 输入代理（回车跳过）
  ✓ 代理可用，连接 x.com 成功 (978ms)
  ✓ chrome Cookies 就绪（提取 247 条）      ← 自动从浏览器提取
  ✓ yt-dlp: 2026.07.04
  ✓ ffmpeg: 可用
```

配置自动保存，下次启动跳过所有提示。

## 交互模式

主菜单只有三个选项：

```
  ━━━━━━━━━━━━ 主菜单 ━━━━━━━━━━━━
  1. 下载视频
  2. 查看配置
  3. 更新 yt-dlp
  0. 退出
```

**下载流程**：输入 URL → 展示格式列表 → 选格式 → 输出目录 → 仅进度条下载。

**格式选择**：

| 选项 | 含义 |
|------|------|
| `b` / 回车 | 最佳质量 |
| `w` | 最佳视频 + 最佳音频（需 ffmpeg） |
| `a` | 仅最佳音频 |
| `0` ~ `N` | 指定格式编号 |

**查看配置**：修改代理、Cookies 来源，立即生效并持久化。

## 命令行模式

```bash
java Main <URL>                          # 下载最佳质量
java Main <URL> -f 18                    # 指定格式
java Main <URL> -f bestaudio -x          # 仅提取 MP3
java Main <URL> -o ./videos              # 指定输出目录
java Main <URL> --info                   # 仅查看信息
java Main <URL> -cb chrome               # 从 Chrome 读 Cookies
java Main <URL> -cb firefox              # 从 Firefox 读 Cookies
java Main <URL> -c cookies.txt           # 使用 Cookies 文件
```

### 完整参数

| 参数 | 说明 |
|------|------|
| `-f, --format <id>` | 格式 ID（`best`, `18`, `bestaudio` 等） |
| `-o, --output <dir>` | 输出目录（默认 `downloads`） |
| `-x, --extract-audio` | 仅提取音频为 MP3 |
| `-p, --proxy <url>` | 代理地址（如 `http://127.0.0.1:7890`） |
| `-cb, --cookies-from-browser <b>` | 浏览器名（chrome/firefox/edge/brave/opera） |
| `-c, --cookies <file>` | Netscape 格式 Cookies 文件 |
| `-r, --retries <n>` | 重试次数（默认 5） |
| `--max-height <n>` | 最大分辨率限制 |
| `--info` | 仅查看信息，不下载 |
| `-h, --help` | 显示帮助 |

### 启动时预设代理

```bash
# 通过 JVM 参数（不需要走到交互提示）
java -Dhttp.proxyHost=127.0.0.1 -Dhttp.proxyPort=7890 \
     -cp out/production/XDownload -Dfile.encoding=UTF-8 \
     Main <URL>
```

## 项目结构

```
XDownload/
├── src/
│   ├── Main.java                      # 入口，命令行解析
│   ├── ui/
│   │   └── ConsoleUI.java             # 交互界面
│   ├── downloader/
│   │   └── YtDlpDownloader.java       # yt-dlp 调用封装
│   ├── model/
│   │   ├── VideoInfo.java             # 视频信息模型
│   │   └── DownloadConfig.java        # 下载配置
│   └── util/
│       ├── ProcessHelper.java          # 进程调用 + Cookies 验证
│       ├── Bootstrap.java             # 首次运行自动下载 yt-dlp/ffmpeg
│       ├── ProxyConfig.java           # 全局代理管理
│       ├── ConfigManager.java         # 配置持久化（JSON）
│       └── ChromeCookies.java         # Chrome DB 备份绕过锁
├── bin/                               # yt-dlp.exe / ffmpeg.exe
├── config/
│   └── settings.json                  # 用户配置（自动生成）
├── downloads/                         # 下载输出（默认）
└── out/production/XDownload/          # 编译输出
```

## 常见问题

### 下载 Twitter/X 视频提示"需要登录"

X.com 的视频需要登录态才能访问。程序默认从 Chrome 读 Cookies，如果：
- **Chrome 正在运行** → 自动尝试 Firefox、Edge，无需手动操作
- **所有浏览器都锁定** → 关闭浏览器后重试，或在选项 2 中手动切换

### 在国内无法访问 YouTube / X

配置代理即可。交互模式启动时会提示输入，或通过 JVM 参数：

```bash
java -Dhttp.proxyHost=127.0.0.1 -Dhttp.proxyPort=7890 ... Main <URL>
```

### yt-dlp 报错"系统找不到指定的文件"

程序会自动修复：`bin/yt-dlp.exe` 存在但 Java 在 Windows 上可能判定为不可执行。启动时会自动重新定位到绝对路径。

### Cookies 提取条数为 0

浏览器中没有目标网站的登录 session。请先在浏览器中登录对应网站。

## 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 下载引擎
- [FFmpeg](https://ffmpeg.org) — 格式转换与合并
