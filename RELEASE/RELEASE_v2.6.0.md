# XDownload v2.6.0 Release Notes

> ✨ v2.6.0 聚焦下载可靠性、历史页体验与界面细节：原子化下载不再残留片段文件、文件名净化更人性化、分阶段下载进度、历史页完整视频信息、重新下载实时解析，并修复「打开文件位置」与下载提示叠加等问题。

---

## ✨ 新功能

### 🗂 下载历史页升级
- 历史卡片与下载页视频信息一致：**封面图**（缺失/失效自动兜底显示应用图标）、可点击标题、下载时间徽标
- 展示**作者 / 时长 / 播放 / 点赞**信息网格
- **重新下载**：自动切到下载页并**重新解析链接**（获取实时格式列表与信息）后自动开始下载
- 「打开文件位置 / 重新下载 / 删除」三个按钮与下载时间徽标**同一行、置右**

### 📊 分阶段下载进度
- 分离流下载时底部进度条依次显示：`下载进度[视频]` → `下载进度[音频]` → `音视频合并`
- 音视频合并/后处理阶段显示**无百分比与速度的循环跑动条**

### 📁 原子化下载（download_cache）
- 下载先写入 `download_cache` 临时目录，**全部完成后才移动到真实下载目录**
- 下载中断 / 取消：自动**清空缓存**，不在下载目录残留 `.part` 片段
- 每次启动自动清理上次异常退出的残留文件

---

## 🐛 修复与优化

- **文件名净化** — 不再 `标题 (1).mp4` 序号重命名；只做「合并连续空格 + 移除 Windows 非法字符 `\ / : * ? " < > |`」，保留中文、括号、emoji；同名文件直接覆盖
- **打开文件位置打不开/错位** — 弃用 `explorer /select`（含空格路径解析不可靠，曾打开 Documents），改用系统 API `SHOpenFolderAndSelectItems`
- **下载提示红绿叠加** — `downloadStore` 集中强制「完成/失败」状态互斥，修复状态栏同时出现红色失败 + 绿色完成两个圆形图标
- **免责声明页** — 「卸载本软件」「前往提交 Issues」按钮置右；修复内容未超出却出现滚动条的问题
- **历史页重新下载** — 不再复用历史占位数据（格式列表为空），改为 yt-dlp **实时解析**后再下载

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

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.6.0) 下载对应平台安装包：

- **Windows**: `XDownload_2.6.0_x64-setup.exe` (NSIS) / `.msi`

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

- 从 v2.5.x 升级**无需迁移配置**
- 历史页旧的下载记录缺少封面/链接/元数据：封面显示应用图标兜底，「重新下载」按钮需先获取视频信息（新记录完整可用）
- 下载缓存目录 `download_cache/` 由应用自动创建与管理，可放心清理

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
