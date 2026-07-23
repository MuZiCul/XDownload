package ui;

import downloader.YtDlpDownloader;
import downloader.YtDlpDownloader.DownloadProgress;
import model.DownloadConfig;
import model.VideoInfo;
import model.VideoInfo.Format;
import util.ProcessHelper;

import java.io.File;
import java.io.IOException;
import java.util.List;
import java.util.Scanner;

/**
 * 控制台交互界面
 */
public class ConsoleUI {

    private final Scanner scanner;
    private final YtDlpDownloader downloader;
    private boolean proxyTested = false;
    private String defaultCookiesFromBrowser = "chrome";  // 全局默认：从浏览器读取cookies
    private String defaultCookiesFile = null;         // 全局默认：cookies文件路径

    public ConsoleUI() {
        this.scanner = new Scanner(System.in);
        this.downloader = new YtDlpDownloader();
    }

    /**
     * 主交互循环
     */
    public void start() {
        printBanner();

        // 加载持久化配置（代理、cookies），存在则自动应用
        loadSavedConfig();
        checkEnvironment();

        // 将默认 cookies 设置应用到下载器，所有操作自动生效
        applyCookiesToDownloader();

        while (true) {
            printMenu();
            System.out.print("\n请输入选项 > ");
            String choice = scanner.nextLine().trim();

            switch (choice) {
                case "1": handleDownload(); break;
                case "2": handleSettings(); break;
                case "3": handleUpdateYtDlp(); break;
                case "0":
                    System.out.println("\n  感谢使用 XDownload，再见！");
                    return;
                default:
                    System.out.println("  [!] 无效选项，请重新输入");
            }
        }
    }

    // ==================== 功能处理 ====================

    private void handleDownload() {
        System.out.print("\n  请输入视频URL: ");
        String url = scanner.nextLine().trim();
        if (url.isEmpty()) return;

        try {
            // 1. 获取视频信息
            System.out.println("\n  [...] 正在获取视频信息...");
            VideoInfo info = downloader.fetchVideoInfo(url);
            System.out.println(info);

            // 2. 选择格式
            DownloadConfig config = selectFormat(info);
            config.setUrl(url);

            // 3. 设置输出目录
            System.out.print("  输出目录 (默认: downloads): ");
            String dir = scanner.nextLine().trim();
            if (!dir.isEmpty()) config.setOutputDir(dir);

            // 4. 是否仅提取音频
            System.out.print("  仅提取音频MP3? (y/n, 默认n): ");
            String audio = scanner.nextLine().trim();
            if ("y".equalsIgnoreCase(audio) || "yes".equalsIgnoreCase(audio)) {
                config.setExtractAudio(true);
            }

            // 5. 开始下载
            doDownload(config);

        } catch (Exception e) {
            System.err.println("  [-] 错误: " + e.getMessage());
        }
    }

    private void handleSettings() {
        System.out.println("\n  --- 当前配置 --");
        System.out.println("  1. yt-dlp 路径: " + ProcessHelper.findYtDlp());
        System.out.println("  2. yt-dlp 版本: " + getYtDlpVersion());
        System.out.println("  3. ffmpeg 状态: " + (ProcessHelper.isFfmpegAvailable() ? "[+] 可用" : "[-] 不可用"));
        System.out.println("  4. 代理状态: " + util.ProxyConfig.getProxyString());
        System.out.println("  5. Cookies来源: " + cookiesSourceDesc());

        System.out.println();
        System.out.println("  操作:");
        System.out.println("    [p] 设置代理");
        System.out.println("    [d] 禁用代理");
        System.out.println("    [t] 测试代理 (通过 x.com 验证)");
        System.out.println("    [cb] 设置从浏览器读取Cookies (chrome/firefox/edge/brave/opera)");
        System.out.println("    [cc] 清除Cookies设置");
        System.out.println("    [回车] 返回");
        System.out.print("  > ");
        String choice = scanner.nextLine().trim().toLowerCase();

        switch (choice) {
            case "p":
                System.out.print("    代理地址 (host:port): ");
                String input = scanner.nextLine().trim();
                if (!input.isEmpty()) {
                    String[] parts = input.split(":");
                    if (parts.length >= 2) {
                        try {
                            String host = parts[0].trim();
                            int port = Integer.parseInt(parts[1].trim());
                            util.ProxyConfig.setProxy(host, port);
                            util.ConfigManager.saveProxy(host, port);
                            testAndShowProxy();
                        } catch (NumberFormatException e) {
                            System.out.println("  [!] 端口号无效");
                        }
                    } else {
                        System.out.println("  [!] 格式: host:port");
                    }
                }
                break;
            case "d":
                util.ProxyConfig.disable();
                removeSavedProxy();
                System.out.println("  [+] 代理已禁用");
                break;
            case "t":
                testAndShowProxy();
                break;
            case "cb":
                System.out.print("    浏览器名称 (chrome/firefox/edge/brave/opera): ");
                String browserName = scanner.nextLine().trim().toLowerCase();
                if (!browserName.isEmpty()) {
                    System.out.print("  [...] 正在验证 " + browserName + " Cookies ...");
                    ProcessHelper.CookiesValidationResult result =
                            ProcessHelper.validateCookiesFromBrowser(browserName);
                    System.out.print("\r  " + result.message + "        \n");
                    if (result.success) {
                        defaultCookiesFromBrowser = browserName;
                        defaultCookiesFile = null;
                        applyCookiesToDownloader();
                        util.ConfigManager.saveCookies(browserName, null);
                    }
                }
                break;
            case "cc":
                defaultCookiesFromBrowser = null;
                defaultCookiesFile = null;
                downloader.setCookiesFromBrowser(null);
                downloader.setCookiesFile(null);
                util.ConfigManager.clearCookies();
                System.out.println("  [+] Cookies设置已清除");
                break;
        }

        System.out.print("  按回车继续...");
        scanner.nextLine();
    }

    private void handleUpdateYtDlp() {
        try {
            downloader.updateYtDlp();
        } catch (Exception e) {
            System.err.println("  [-] 更新失败: " + e.getMessage());
        }
    }

    // ==================== 辅助方法 ====================

    private DownloadConfig selectFormat(VideoInfo info) {
        List<Format> formats = info.getFormats();

        if (formats.isEmpty()) {
            System.out.println("  [!] 未找到可用格式，使用默认最佳格式");
            return new DownloadConfig();
        }

        System.out.println("\n  选择下载方式:");
        System.out.println("    [b] 最佳质量 (默认)");
        System.out.println("    [w] 最佳视频+最佳音频 (需ffmpeg合并)");
        System.out.println("    [a] 仅最佳音频");
        System.out.print("    [0-" + (formats.size() - 1) + "] 指定格式编号\n");
        System.out.print("  请选择 > ");

        String choice = scanner.nextLine().trim().toLowerCase();
        DownloadConfig config = new DownloadConfig();

        switch (choice) {
            case "b":
            case "":
                config.setFormatId("best");
                break;
            case "w":
                config.setFormatId("bestvideo+bestaudio/best");
                break;
            case "a":
                config.setFormatId("bestaudio");
                config.setExtractAudio(true);
                break;
            default:
                try {
                    int idx = Integer.parseInt(choice);
                    if (idx >= 0 && idx < formats.size()) {
                        config.setFormatId(formats.get(idx).getFormatId());
                    } else {
                        System.out.println("  [!] 编号无效，使用最佳格式");
                        config.setFormatId("best");
                    }
                } catch (NumberFormatException e) {
                    System.out.println("  [!] 输入无效，使用最佳格式");
                    config.setFormatId("best");
                }
        }

        return config;
    }

    private boolean doDownload(DownloadConfig config) throws IOException, InterruptedException {
        // 进度显示
        final int[] lastPercent = { -1 };
        final long[] startTime = { System.currentTimeMillis() };

        System.out.println();
        boolean result = downloader.download(config, progress -> {
            int pct = (int) progress.getPercentValue();
            if (pct != lastPercent[0] || pct == 100) {
                lastPercent[0] = pct;
                // 清除当前行并打印进度条
                System.out.print("\r  " + progress.toString());
            }
            if ("finished".equals(progress.status) || "downloading".equals(progress.status)) {
                // 正常状态
            }
        });

        long elapsed = System.currentTimeMillis() - startTime[0];
        System.out.println(); // 换行

        if (result) {
            System.out.println("  [+] 下载完成! 耗时: " + formatElapsed(elapsed));
            System.out.println("    保存位置: " + new File(config.getOutputDir()).getAbsolutePath());
        }

        return result;
    }

    private void printBanner() {
        System.out.println();
        System.out.println("  ==========================================");
        System.out.println("         XDownload - X视频下载工具  v" + util.Version.CURRENT);
        System.out.println("         基于 yt-dlp  | By MuZiCul           ");
        System.out.println("  ==========================================");
        System.out.println();
    }

    private void printMenu() {
        System.out.println();
        System.out.println("  ------------ 主菜单 ------------");
        System.out.println("  1. 下载视频");
        System.out.println("  2. 查看配置");
        System.out.println("  3. 更新 yt-dlp");
        System.out.println("  0. 退出");
    }

    /** 从 config/settings.json 加载持久化配置 */
    private void loadSavedConfig() {
        // 加载代理（如果还没通过 JVM 参数设置）
        if (!util.ProxyConfig.isEnabled()) {
            util.ConfigManager.applySavedProxy();
        }

        // 加载 cookies 默认值
        if (defaultCookiesFromBrowser == null && defaultCookiesFile == null) {
            String[] saved = util.ConfigManager.loadSavedCookies();
            if (saved[0] != null) {
                defaultCookiesFromBrowser = saved[0];
            } else if (saved[1] != null) {
                defaultCookiesFile = saved[1];
            }
        }
    }

    private void checkEnvironment() {
        System.out.println("  --- 环境检查 ---");

        // ===== 第1步：检测国内外环境 → 是否需要代理 =====
        boolean hasSavedProxy = util.ProxyConfig.isEnabled();
        if (!hasSavedProxy) {
            System.out.print("  [...] 检测网络环境... (回车跳过) ");
            Boolean overseas = detectOverseasWithSkip();
            if (overseas == null) {
                System.out.println("\r  [!] 已跳过检测，请手动配置代理        ");
                promptForProxy();
            } else if (overseas) {
                System.out.println("\r  [+] 海外环境，无需代理        ");
            } else {
                System.out.println("\r  [!] 国内环境，需配置代理以访问外网        ");
                promptForProxy();
            }
        } else {
            System.out.println("  [+] 代理: " + util.ProxyConfig.getProxyString() + " (已从配置加载)");
            if (!proxyTested) testAndShowProxy();
        }

        // ===== 第2步：检查 yt-dlp（必须）=====
        if (ProcessHelper.isYtDlpAvailable()) {
            System.out.println("  [+] yt-dlp: " + getYtDlpVersion()
                    + "  (" + ProcessHelper.findYtDlp() + ")");
        } else {
            System.out.print("  [!] yt-dlp 不存在，是否自动下载？(约 17MB, y/n, 默认y): ");
            String choice = scanner.nextLine().trim();
            if (choice.isEmpty() || "y".equalsIgnoreCase(choice) || "yes".equalsIgnoreCase(choice)) {
                try {
                    String path = util.Bootstrap.ensureYtDlp();
                    System.out.println("  [+] yt-dlp 已就绪: " + getYtDlpVersion());
                } catch (Exception e) {
                    System.out.println("  [-] yt-dlp 下载失败！");
                    System.out.println("    手动: winget install yt-dlp.yt-dlp");
                    System.out.println("    或 https://github.com/yt-dlp/yt-dlp/releases");
                    System.out.println();
                    System.out.print("  按回车退出...");
                    scanner.nextLine();
                    System.exit(1);
                }
            } else {
                System.out.println("  [-] yt-dlp 是必须组件，无法继续。");
                System.exit(1);
            }
        }

        // ===== 第3步：检查 ffmpeg（可选）=====
        if (ProcessHelper.isFfmpegAvailable()) {
            System.out.println("  [+] ffmpeg: 可用 (支持格式转换与合并)");
        } else {
            System.out.print("  [!] ffmpeg 不存在，是否自动下载？(约 80MB, y/n, 默认n): ");
            String choice = scanner.nextLine().trim();
            if ("y".equalsIgnoreCase(choice) || "yes".equalsIgnoreCase(choice)) {
                try {
                    util.Bootstrap.ensureFfmpeg(true);
                    if (ProcessHelper.isFfmpegAvailable()) {
                        System.out.println("  [+] ffmpeg 已就绪");
                    }
                } catch (Exception e) {
                    System.out.println("  [!] ffmpeg 下载失败（不影响基本下载）: " + e.getMessage());
                }
            } else {
                System.out.println("    提示: 可手动 winget install ffmpeg  或 https://ffmpeg.org");
            }
        }

        // ===== 第4步：导入 Cookies =====
        if (defaultCookiesFromBrowser == null && defaultCookiesFile == null) {
            promptForCookies();
        } else {
            // 已有配置，验证并可能回退
            String chosen = resolveWorkingBrowser(defaultCookiesFromBrowser);
            if (!chosen.equals(defaultCookiesFromBrowser)) {
                defaultCookiesFromBrowser = chosen;
                applyCookiesToDownloader();
            }
        }

        System.out.println();
    }

    /**
     * 异步检测国内外环境，用户可按回车跳过
     * @return true=海外, false=国内, null=用户跳过
     */
    private Boolean detectOverseasWithSkip() {
        final boolean[] done = {false};
        final boolean[] result = {false};
        // 后台线程：网络检测
        Thread detector = new Thread(() -> {
            result[0] = util.NetworkDetect.isOverseas();
            done[0] = true;
        }, "net-detect");
        detector.setDaemon(true);
        detector.start();

        // 主线程：等待检测完成或用户按回车（最多 3s）
        long deadline = System.currentTimeMillis() + 3000;
        while (System.currentTimeMillis() < deadline && !done[0]) {
            try {
                if (System.in.available() > 0) {
                    // 用户按了回车，消费掉输入
                    scanner.nextLine();
                    return null;
                }
                Thread.sleep(100);
            } catch (Exception e) {
                break;
            }
        }
        // 等待检测线程结束（最多再给 1s）
        if (!done[0]) {
            try { detector.join(1000); } catch (InterruptedException ignored) {}
        }
        return done[0] ? result[0] : false; // 超时默认按国内处理
    }

    /** 询问并验证代理 */
    private void promptForProxy() {
        System.out.print("  代理地址 (host:port, 如 127.0.0.1:7890, 回车跳过): ");
        String proxyInput = scanner.nextLine().trim();
        if (!proxyInput.isEmpty()) {
            try {
                String[] parts = proxyInput.split(":");
                if (parts.length >= 2) {
                    String pHost = parts[0].trim();
                    int pPort = Integer.parseInt(parts[1].trim());
                    util.ProxyConfig.setProxy(pHost, pPort);
                    util.ConfigManager.saveProxy(pHost, pPort);
                    testAndShowProxy();
                    return;
                } else {
                    System.out.println("  [!] 格式错误，应为 host:port");
                }
            } catch (NumberFormatException e) {
                System.out.println("  [!] 端口号无效");
            }
        }
        if (!util.ProxyConfig.isEnabled()) {
            System.out.println("     (提示: 也可通过 -Dhttp.proxyHost=IP -Dhttp.proxyPort=PORT 启动)");
        }
    }

    /** 询问并导入 Cookies */
    private void promptForCookies() {
        System.out.print("  [...] 扫描本地浏览器 Cookies ...");
        String found = scanForCookies();
        if (found != null) {
            System.out.print("\r  [+] 检测到 " + found + " 浏览器已登录，是否导入？(y/n, 默认y): ");
            String choice = scanner.nextLine().trim();
            if (choice.isEmpty() || "y".equalsIgnoreCase(choice) || "yes".equalsIgnoreCase(choice)) {
                defaultCookiesFromBrowser = found;
                util.ConfigManager.saveCookies(found, null);
                applyCookiesToDownloader();
                // 验证
                ProcessHelper.CookiesValidationResult r =
                        ProcessHelper.validateCookiesFromBrowser(found);
                System.out.println("  " + r.message);
                return;
            }
        } else {
            System.out.println("\r  [!] 未检测到可用浏览器 Cookies        ");
        }
        System.out.print("  手动输入浏览器名导入 (chrome/firefox/edge, 回车跳过): ");
        String input = scanner.nextLine().trim().toLowerCase();
        if (!input.isEmpty()) {
            ProcessHelper.CookiesValidationResult r =
                    ProcessHelper.validateCookiesFromBrowser(input);
            System.out.println("  " + r.message);
            if (r.success) {
                defaultCookiesFromBrowser = input;
                util.ConfigManager.saveCookies(input, null);
                applyCookiesToDownloader();
            }
        }
    }

    /** 扫描本地浏览器，返回第一个可读取 cookies 的浏览器名 */
    private String scanForCookies() {
        String[] browsers = {"chrome", "firefox", "edge", "brave", "opera"};
        for (String browser : browsers) {
            ProcessHelper.CookiesValidationResult r =
                    ProcessHelper.validateCookiesFromBrowser(browser);
            if (r.success && r.cookieCount > 0) return browser;
        }
        return null;
    }

    private String getYtDlpVersion() {
        try {
            ProcessHelper.CommandResult result = ProcessHelper.execute(
                    List.of(ProcessHelper.findYtDlp(), "--version"));
            if (result.isSuccess() && !result.stdout.isEmpty()) {
                return result.stdout.get(0).trim();
            }
        } catch (Exception ignored) {}
        return "未知";
    }

    /**
     * 测试代理并显示结果
     */
    private void testAndShowProxy() {
        proxyTested = true;
        System.out.print("  [...] 正在通过 x.com 验证代理 ...");
        util.ProxyConfig.ProxyTestResult result = util.ProxyConfig.testProxy();
        System.out.print("\r  " + result.toString() + "        \n");
        if (!result.success) {
            System.out.println("     TIP: 建议检查代理地址和端口是否正确");
        }
    }

    private String cookiesSourceDesc() {
        if (defaultCookiesFromBrowser != null) return "浏览器 " + defaultCookiesFromBrowser;
        if (defaultCookiesFile != null) return "文件 " + defaultCookiesFile;
        return "未设置";
    }

    /** 浏览器回退列表 */
    private static final String[] FALLBACK_BROWSERS = {"chrome", "firefox", "edge", "brave", "opera"};

    /** 逐个尝试浏览器，返回第一个可用的 */
    private String resolveWorkingBrowser(String preferred) {
        // 先试首选
        System.out.print("  [...] 验证 " + preferred + " Cookies ...");
        ProcessHelper.CookiesValidationResult r =
                ProcessHelper.validateCookiesFromBrowser(preferred);
        System.out.print("\r  " + r.message + "        \n");
        if (r.success) return preferred;

        // 回退
        for (String browser : FALLBACK_BROWSERS) {
            if (browser.equalsIgnoreCase(preferred)) continue;
            System.out.print("  [...] 尝试 " + browser + " Cookies ...");
            r = ProcessHelper.validateCookiesFromBrowser(browser);
            System.out.print("\r  " + r.message + "        \n");
            if (r.success) {
                System.out.println("  [!] " + preferred + " 不可用，自动切换至 " + browser);
                return browser;
            }
        }
        System.out.println("     TIP: 所有浏览器 Cookies 均不可用（请关闭浏览器后重试）");
        return preferred; // 全部失败，保留原设置
    }

    /** 从持久化配置中移除代理 */
    private void removeSavedProxy() {
        util.ConfigManager.removeProxy();
    }

    /** 将默认 cookies 配置应用到下载器（所有操作自动携带） */
    private void applyCookiesToDownloader() {
        if (defaultCookiesFromBrowser != null) {
            downloader.setCookiesFromBrowser(defaultCookiesFromBrowser);
        } else if (defaultCookiesFile != null) {
            downloader.setCookiesFile(defaultCookiesFile);
        }
    }

    private String formatElapsed(long millis) {
        long seconds = millis / 1000;
        if (seconds < 60) return seconds + "秒";
        long minutes = seconds / 60;
        seconds = seconds % 60;
        return minutes + "分" + seconds + "秒";
    }
}
