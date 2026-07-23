# XDownload v1.0.5

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的 Windows 便携视频下载器，开箱即用，无需安装任何依赖。

## 特性

- **便携免安装** — 打包为单目录 exe，内置 JRE，U 盘随身携带
- **国内外自动感知** — 启动时检测网络环境，海外自动跳过代理，国内引导配置代理
- **浏览器 Cookies 直读** — 自动扫描 Chrome / Firefox / Edge / Brave / Opera 登录态
- **智能回退** — 当前浏览器 Cookies 不可用时自动切换下一个
- **配置持久化** — 代理、Cookies 偏好保存至 `config/settings.json`，下次自动加载
- **纯控制台** — 下载时仅显示进度条，干净无冗余输出
- **交互 + CLI 双模式** — 菜单引导和命令行一键下载都支持

## 快速开始

### 方式一：便携版（推荐）

1. 双击 `build.bat` 构建
2. 将 `build\XDownload\` 复制到任意位置
3. 双击 `XDownload.exe`

### 方式二：Java 运行

```bash
javac -encoding UTF-8 -d out/production/XDownload -sourcepath src src/Main.java
java -cp out/production/XDownload -Dfile.encoding=UTF-8 Main
```

## 首次启动

```
  ==========================================
         XDownload - X视频下载工具  v1.0.5
         基于 yt-dlp  | By MuZiCul
  ==========================================

  --- 环境检查 ---
  [...] 检测网络环境... (回车跳过)          ← 3 秒自动判断，可回车跳过
  [+] 海外环境，无需代理                     ← 国外直接下一步

  [+] yt-dlp: 2026.07.04                   ← 已存在则跳过
  [+] ffmpeg: 可用                          ← 不存在则询问下载

  [...] 扫描本地浏览器 Cookies ...
  [+] 检测到 chrome 浏览器已登录，是否导入？  ← 自动扫描
  [+] chrome Cookies 就绪（提取 247 条）      ← 导入后验证
```

国内环境会提示：

```
  [!] 国内环境，需配置代理以访问外网
  代理地址 (host:port): 127.0.0.1:7890
  [+] 代理可用，连接 x.com 成功 (230ms)
```

## 交互模式

```
  ------------ 主菜单 ------------
  1. 下载视频
  2. 查看配置
  3. 更新 yt-dlp
  0. 退出
```

**下载流程**：输入 URL → 展示格式列表 → 选格式 → 输出目录 → 进度条下载

**格式选择**：

| 选项 | 含义 |
|------|------|
| `b` / 回车 | 最佳质量 |
| `w` | 最佳视频 + 最佳音频 |
| `a` | 仅最佳音频 |
| `0` ~ `N` | 指定格式编号 |

## 命令行模式

```bash
java Main <URL> --info                   # 仅查看信息
java Main <URL> -f 18                    # 指定格式
java Main <URL> -f bestaudio -x          # 仅提取 MP3
java Main <URL> -o ./videos              # 指定输出目录
java Main <URL> -cb chrome               # 从 Chrome 读 Cookies
java Main <URL> -cb firefox              # 从 Firefox 读 Cookies
java Main <URL> -p http://127.0.0.1:7890 # 指定代理
```

### 完整参数

| 参数 | 说明 |
|------|------|
| `-f, --format <id>` | 格式 ID |
| `-o, --output <dir>` | 输出目录（默认 `downloads`） |
| `-x, --extract-audio` | 仅提取音频 MP3 |
| `-p, --proxy <url>` | 代理地址 |
| `-cb, --cookies-from-browser <b>` | 浏览器（chrome/firefox/edge/brave/opera） |
| `-c, --cookies <file>` | Netscape 格式 Cookies 文件 |
| `-r, --retries <n>` | 重试次数（默认 5） |
| `--max-height <n>` | 最大分辨率 |
| `--info` | 仅查看信息 |
| `-h, --help` | 帮助 |
| `-v, --version` | 版本 |

### JVM 参数预设代理

```bash
java -Dhttp.proxyHost=127.0.0.1 -Dhttp.proxyPort=7890 ... Main <URL>
```

## 构建便携版

```bash
build.bat
```

构建过程：

1. `javac` 编译源码
2. `jar` 打包
3. `jlink` 剪裁定制 JRE（仅保留必要模块，约 40MB）
4. `jpackage` 生成原生 Windows exe（app-image）
5. 复制 `bin/yt-dlp.exe`、`bin/ffmpeg.exe` 到镜像

输出目录 `build\XDownload\` 结构：

```
XDownload\
├── XDownload.exe          ← 双击启动
├── app\                   ← 主程序 JAR
├── runtime\               ← 内置 JRE
├── bin\                   ← yt-dlp.exe / ffmpeg.exe
├── config\                ← 配置文件（自动生成）
└── downloads\             ← 下载输出
```

总大小约 140MB（含 ffmpeg），压缩后约 50MB。

## 项目结构

```
XDownload/
├── src/
│   ├── Main.java                      # 入口 + CLI 解析
│   ├── ui/
│   │   └── ConsoleUI.java             # 交互界面 + 启动引导
│   ├── downloader/
│   │   └── YtDlpDownloader.java       # yt-dlp 调用封装
│   ├── model/
│   │   ├── VideoInfo.java             # 视频信息 + 格式模型
│   │   └── DownloadConfig.java        # 下载配置
│   └── util/
│       ├── AppHome.java               # 应用根目录解析
│       ├── ProcessHelper.java         # 进程调用 + Cookies 验证
│       ├── Bootstrap.java             # 首次运行自动下载依赖
│       ├── ProxyConfig.java           # 代理管理
│       ├── ConfigManager.java         # 配置持久化
│       ├── ChromeCookies.java         # Chrome DB 备份
│       ├── NetworkDetect.java         # 国内外网络检测
│       └── Version.java               # 版本号
├── bin/                               # yt-dlp.exe / ffmpeg.exe
├── build.bat                          # 便携版构建脚本
└── README.md
```

## 常见问题

### 下载 X/Twitter 视频提示需要登录

程序自动从浏览器提取 Cookies，如果 Chrome 正运行可能锁库，会依次尝试 Firefox → Edge 等。

### 国内无法访问外网

启动时自动检测，国内环境会引导输入代理地址，配置后持久化保存。

### exe 的 TLS 握手失败

已修复：jlink 包含 `jdk.crypto.ec` 模块支持现代 ECDHE 加密套件。

### 控制台图标乱码

已全部替换为 ASCII 字符（`[+]` `[-]` `[!]` `[...]`），兼容所有 Windows 终端。

## 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [FFmpeg](https://ffmpeg.org)
