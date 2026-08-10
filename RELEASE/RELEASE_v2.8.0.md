# XDownload v2.8.0 Release Notes

> 🛠 v2.8.0 重点完善**浏览器扩展**下载链路，新增**隐私模式**。配套扩展 **v1.0.0** 在 X 推文上直接添加下载按钮，一键把视频送入桌面端下载。

---

## 🌐 浏览器扩展（重点）

**XDownload Browser Extension v1.0.0** 是配套的 MV3 扩展，**兼容 Chrome / Edge**（同为 Chromium 内核，行为一致，商店与开发者模式加载通用）。

### 它能做什么

- 在 X（twitter.com / x.com）页面**每个推文右下角自动注入「下载」按钮**
- 点击后通过 `xdownload://` 协议把视频地址送入桌面端 XDownload
- 桌面端自动入队、获取信息并开始下载，同时 toast「已从浏览器获得下载任务」并跳转到任务页
- 应用未启动时也会自动拉起 XDownload 后再入队（单实例转发）

### 安装方式

1. 下载本 Release 中的 **`xdownload-extension-v1.0.0.zip`**
2. 解压到本地任意文件夹
3. Chrome / Edge 打开扩展管理页（`chrome://extensions` 或 `edge://extensions`）
4. 右上角开启「开发者模式」→ 「加载已解压的扩展程序」→ 选择解压后的文件夹

> 后续计划上架 Edge Add-ons / Chrome Web Store，届时可直接商店安装、自动更新。

### v2.8.0 配套深链修复

- **URL 规范化**：浏览器把 `xdownload://add?url=` 空路径规范化为 `add/` 导致解析被拒，现容忍尾部斜杠
- **先 fetch 再入队**：深链任务先获取视频信息再入队（与普通 UI 流程一致），任务卡片与下载历史信息完整
- **下载无历史记录**：深链入队补充 `video_id`（从 status URL 提取），历史记录写入恢复正常
- **下载目录错误**：相对路径 `downloads` 不再按进程 cwd 解析（协议拉起时 cwd 为 system32 曾导致文件下载到 `system32\downloads`），统一基于应用根目录
- **入队提示**：添加任务后 toast「已从浏览器获得下载任务」并自动跳转任务页

---

## ✨ 其他新增

- **隐私模式**
  - 设置页「软件日志」旁、右下角状态栏、系统托盘菜单三个入口均可开启/退出
  - 开启后任务页（正在下载/下载完成）与下载页已获取信息的**标题以 `***` 显示、封面毛玻璃覆盖**
  - 状态持久化到 `config/settings.json`，重启后保持；托盘菜单初始文本启动即正确
- **下载完成 toast 点击打开文件位置**：toast「下载完成 + 标题」，点击标题在资源管理器定位到该视频文件（新增 `open_file_path` 命令）
- **`debug_build.bat` 快速调试构建脚本**：debug 模式 + 跳过安装包打包，加速本地调试迭代

---

## 📦 安装

### 下载预构建包

从 [GitHub Releases](https://github.com/MuZiCul/XDownload/releases/tag/v2.8.0) 下载对应平台安装包：

- **Windows**: `XDownload_2.8.0_x64-setup.exe` (NSIS) / `.msi`
- **浏览器扩展**: `xdownload-extension-v1.0.0.zip`（见上方「浏览器扩展」章节）

### 从源码构建

```bash
# 环境要求：Rust (MSVC)、Node.js 18+、pnpm

# 开发模式
pnpm dev

# 打包构建
.\build.bat          # Windows

# 快速调试构建（不打包安装程序）
.\debug_build.bat
```

---

## ⚠️ 升级说明

- 从 v2.7.2 升级**无需迁移配置**，直接覆盖安装即可
- 浏览器扩展 v1.0.0（协议格式不变，无需重装）；桌面端首次启动自动注册 `xdownload://` 协议

---

## 🙏 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 视频解析与下载引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架
- [ffmpeg](https://ffmpeg.org/) — 音视频处理
- [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) — Windows ffmpeg 构建
