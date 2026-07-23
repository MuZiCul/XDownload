package downloader;

import model.DownloadConfig;
import model.VideoInfo;
import model.VideoInfo.Format;
import util.ProcessHelper;
import util.ProcessHelper.CommandResult;
import util.ProxyConfig;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * 核心下载器，封装 yt-dlp（支持 1000+ 视频网站）
 * <p>
 * 基于开源项目 yt-dlp (https://github.com/yt-dlp/yt-dlp)，
 * 通过命令行调用实现视频解析与下载。
 */
public class YtDlpDownloader {

    private final String ytDlpPath;
    private String cookiesFromBrowser;   // 全局：从浏览器读取 cookies
    private String cookiesFile;          // 全局：cookies 文件路径
    private volatile Process currentProcess; // 当前运行的 yt-dlp 进程，供取消使用

    public YtDlpDownloader() {
        this.ytDlpPath = ProcessHelper.findYtDlp();
    }

    /** 取消当前下载（强制终止 yt-dlp 进程） */
    public void cancel() {
        Process p = currentProcess;
        if (p != null && p.isAlive()) {
            p.destroyForcibly();
        }
    }

    public YtDlpDownloader(String ytDlpPath) {
        this.ytDlpPath = ytDlpPath;
    }

    /** 设置从浏览器读取 cookies（每次操作自动生效） */
    public void setCookiesFromBrowser(String browser) {
        this.cookiesFromBrowser = browser;
        this.cookiesFile = null;
    }

    /** 设置 cookies 文件路径（每次操作自动生效） */
    public void setCookiesFile(String path) {
        this.cookiesFile = path;
        this.cookiesFromBrowser = null;
    }

    public String getCookiesFromBrowser() { return cookiesFromBrowser; }
    public String getCookiesFile() { return cookiesFile; }

    // ==================== 视频信息解析 ====================

    /**
     * 获取视频信息（含所有可用格式）
     */
    public VideoInfo fetchVideoInfo(String url) throws IOException, InterruptedException {
        VideoInfo info = new VideoInfo(url);

        resolveCookiesBrowser();
        List<String> cmd = buildBaseCommand();
        cmd.add("--dump-json");          // JSON 格式输出
        cmd.add("--no-playlist");        // 不解析播放列表
        cmd.add(url);

        CommandResult result = executeCookiesWithRetry(cmd, 30);
        if (!result.isSuccess()) {
            String stderr = result.getStderrText();
            if (stderr.contains("age") || stderr.contains("login") || stderr.contains("unavailable")) {
                throw new IOException("需要登录或年龄验证，请通过选项5设置Cookies:\n"
                        + "  - 从浏览器读取: --cookies-from-browser chrome\n"
                        + "  - 或导出cookies文件: --cookies cookies.txt");
            }
            throw new IOException("yt-dlp 解析失败: " + stderr);
        }
        String json = result.getStdoutText();
        if (json.isEmpty()) {
            throw new IOException("无法获取视频信息，请检查URL是否正确");
        }

        parseVideoJson(info, json);
        return info;
    }

    /**
     * 获取播放列表信息
     */
    public List<VideoInfo> fetchPlaylist(String url) throws IOException, InterruptedException {
        List<VideoInfo> videos = new ArrayList<>();

        List<String> cmd = buildBaseCommand();
        cmd.add("--dump-json");
        cmd.add("--flat-playlist");
        cmd.add("--yes-playlist");
        cmd.add(url);

        CommandResult result = ProcessHelper.executeWithTimeout(cmd, 60);
        if (!result.isSuccess()) {
            throw new IOException("播放列表解析失败: " + result.getStderrText());
        }

        for (String line : result.stdout) {
            try {
                VideoInfo info = new VideoInfo(url);
                parseVideoJson(info, line);
                videos.add(info);
            } catch (Exception ignored) {
                // 跳过解析失败的行
            }
        }

        return videos;
    }

    // ==================== 下载 ====================

    /**
     * 下载视频（使用默认最佳格式）
     */
    public boolean download(DownloadConfig config) throws IOException, InterruptedException {
        return download(config, null);
    }

    /**
     * 下载视频，带进度回调
     */
    public boolean download(DownloadConfig config, Consumer<DownloadProgress> progressCallback)
            throws IOException, InterruptedException {

        resolveCookiesBrowser();
        List<String> cmd = buildBaseCommand();
        cmd.add("-f"); cmd.add(config.getFormatId());
        cmd.add("-o"); cmd.add(config.getOutputPath());
        cmd.add("--retries"); cmd.add(String.valueOf(config.getRetries()));
        cmd.add("--socket-timeout"); cmd.add(String.valueOf(config.getSocketTimeout()));
        cmd.add("--no-playlist");

        // 输出目录不存在则创建
        File outDir = new File(config.getOutputDir());
        if (!outDir.exists()) outDir.mkdirs();

        // 下载归档（去重）
        if (config.getDownloadArchive() != null && !config.getDownloadArchive().isEmpty()) {
            cmd.add("--download-archive");
            cmd.add(config.getDownloadArchive());
        }

        // 仅提取音频
        if (config.isExtractAudio()) {
            cmd.add("-x");
            cmd.add("--audio-format"); cmd.add("mp3");
            cmd.add("--audio-quality"); cmd.add("0");
        }

        // 字幕
        if (config.isEmbedSubtitles()) {
            cmd.add("--embed-subs");
            cmd.add("--write-auto-subs");
        }

        // 缩略图
        if (config.isEmbedThumbnail()) {
            cmd.add("--embed-thumbnail");
        }
        if (config.isWriteThumbnail()) {
            cmd.add("--write-thumbnail");
        }

        // 单次下载级别的代理（仅当与全局代理不同时才添加，覆盖 buildBaseCommand 中的设置）
        if (config.getProxy() != null && !config.getProxy().isEmpty()) {
            cmd.add("--proxy"); cmd.add(config.getProxy());
        }

        // 单次下载级别的 Cookies（覆盖全局设置）
        if (config.getCookiesFromBrowser() != null && !config.getCookiesFromBrowser().isEmpty()
                && !config.getCookiesFromBrowser().equals(cookiesFromBrowser)) {
            cmd.add("--cookies-from-browser"); cmd.add(config.getCookiesFromBrowser());
        } else if (config.getCookiesFile() != null && !config.getCookiesFile().isEmpty()
                && !config.getCookiesFile().equals(cookiesFile)) {
            cmd.add("--cookies"); cmd.add(config.getCookiesFile());
        }

        // 最大分辨率限制
        if (config.getMaxHeight() > 0) {
            cmd.add("--format-sort"); cmd.add("height:" + config.getMaxHeight());
        }

        // 进度信息（新版 yt-dlp 输出到 stdout）
        cmd.add("--newline");
        cmd.add("--progress");
        cmd.add("--progress-template");
        cmd.add("%(progress.downloaded_bytes)s|%(progress.total_bytes)s|"
                + "%(progress._speed_str)s|%(progress._eta_str)s|"
                + "%(progress._percent_str)s|%(progress.status)s");

        cmd.add(config.getUrl());

        java.util.concurrent.atomic.AtomicReference<Process> processRef =
                new java.util.concurrent.atomic.AtomicReference<>();
        CommandResult result = ProcessHelper.execute(cmd,
                stdoutLine -> {
                    if (progressCallback != null && stdoutLine.contains("|")) {
                        try {
                            DownloadProgress progress = parseProgress(stdoutLine);
                            if (progress != null) progressCallback.accept(progress);
                        } catch (Exception ignored) {}
                    }
                },
                stderrLine -> {
                    if (stderrLine.contains("ERROR") || stderrLine.contains("error")) {
                        System.err.println(stderrLine);
                    }
                },
                processRef);
        currentProcess = processRef.get();

        if (!result.isSuccess()) {
            System.err.println("[错误] 下载失败 (退出码: " + result.exitCode + ")");
            return false;
        }

        return true;
    }

    /**
     * 更新 yt-dlp 自身（带进度条）
     */
    public boolean updateYtDlp() throws IOException, InterruptedException {
        System.out.println("  [...] 正在检查 yt-dlp 更新 ...");

        final long[] startTime = {System.currentTimeMillis()};
        final int[] lastPct = {-1};
        final boolean[] hasNewVersion = {false};

        CommandResult result = ProcessHelper.execute(
                List.of(ytDlpPath, "-U", "--no-color"),
                stdoutLine -> {
                    // 版本 / 状态信息
                    String lower = stdoutLine.toLowerCase();
                    if (lower.contains("current") || lower.contains("latest")
                            || lower.contains("version") || lower.contains("up to date")
                            || lower.contains("updated")) {
                        System.out.println("  " + stdoutLine);
                    }
                },
                stderrLine -> {
                    // yt-dlp 的更新下载进度也在 stderr，格式类似视频下载
                    // 典型: [download] 12.3% of ~15.00MiB at 2.50MiB/s ETA 00:05
                    if (stderrLine.contains("%")) {
                        hasNewVersion[0] = true;
                        int pct = extractPercentage(stderrLine);
                        if (pct >= 0 && pct != lastPct[0]) {
                            lastPct[0] = pct;
                            printUpdateBar(pct, stderrLine, startTime[0]);
                        }
                    }
                });

        long elapsed = System.currentTimeMillis() - startTime[0];
        System.out.println(); // 换行

        if (result.isSuccess()) {
            if (hasNewVersion[0]) {
                System.out.println("  [+] yt-dlp 更新完成! 耗时: " + formatElapsedTime(elapsed));
            } else {
                System.out.println("  [+] yt-dlp 已是最新版本");
            }
            return true;
        } else {
            System.out.println("  [-] 更新失败 (退出码: " + result.exitCode + ")");
            if (!result.getStderrText().isEmpty()) {
                System.out.println("     " + result.getStderrText());
            }
            return false;
        }
    }

    /**
     * 从更新输出行中提取百分比
     * 支持格式: "12.3%", "[download] 12.3%" 等
     */
    private static int extractPercentage(String line) {
        Pattern p = Pattern.compile("(\\d+\\.?\\d*)\\s*%");
        Matcher m = p.matcher(line);
        if (m.find()) {
            try {
                return (int) Double.parseDouble(m.group(1));
            } catch (NumberFormatException ignored) {}
        }
        return -1;
    }

    /**
     * 打印更新进度条
     */
    private static void printUpdateBar(int percent, String contextLine, long startTime) {
        int barLen = 30;
        int filled = percent * barLen / 100;

        StringBuilder bar = new StringBuilder("\r  [");
        for (int i = 0; i < barLen; i++) {
            if (i < filled) bar.append("=");
            else if (i == filled) bar.append(">");
            else bar.append(" ");
        }
        bar.append("] ");
        bar.append(String.format("%3d%%", percent));

        // 尝试提取速度和 ETA
        Pattern speedP = Pattern.compile("at\\s+(\\S+)/s");
        Matcher speedM = speedP.matcher(contextLine);
        if (speedM.find()) {
            bar.append(" | ").append(speedM.group(1)).append("/s");
        }

        Pattern etaP = Pattern.compile("ETA\\s+(\\S+)");
        Matcher etaM = etaP.matcher(contextLine);
        if (etaM.find()) {
            bar.append(" | ETA ").append(etaM.group(1));
        }

        // 已用时间
        long elapsed = (System.currentTimeMillis() - startTime) / 1000;
        bar.append(" | 耗时 ").append(elapsed).append("s");

        System.out.print(bar.toString());
    }

    private static String formatElapsedTime(long millis) {
        long seconds = millis / 1000;
        if (seconds < 60) return seconds + "秒";
        long minutes = seconds / 60;
        seconds = seconds % 60;
        if (minutes < 60) return minutes + "分" + seconds + "秒";
        return (minutes / 60) + "时" + (minutes % 60) + "分";
    }

    // ==================== 私有方法 ====================

    /** 浏览器回退顺序 */
    private static final String[] BROWSER_FALLBACK = {"chrome", "firefox", "edge", "brave", "opera"};

    private boolean cookiesResolved = false;

    /**
     * 自动选择可用的浏览器 cookies，优先使用用户配置的，失败则逐个回退
     */
    private void resolveCookiesBrowser() {
        if (cookiesResolved) return;
        if (cookiesFromBrowser == null && cookiesFile == null) return;
        if (cookiesFile != null) { cookiesResolved = true; return; }

        String preferred = cookiesFromBrowser;

        // 先试首选浏览器
        if (tryBrowserCookies(preferred)) {
            cookiesResolved = true;
            return;
        }

        // 回退：遍历其他浏览器
        for (String browser : BROWSER_FALLBACK) {
            if (browser.equalsIgnoreCase(preferred)) continue;
            if (canReadCookies(browser)) {
                System.out.println("  [!] " + preferred + " Cookies 不可用，自动切换至 " + browser);
                cookiesFromBrowser = browser;
                cookiesResolved = true;
                return;
            }
        }
    }

    /** 尝试预览 Chrome Cookies DB（绕过锁问题） */
    private boolean tryBrowserCookies(String browser) {
        if (browser.equalsIgnoreCase("chrome")) {
            for (int attempt = 0; attempt < 3; attempt++) {
                if (util.ChromeCookies.backupCookiesDb() != null) return true;
                if (attempt < 2) {
                    try { Thread.sleep(1000); } catch (InterruptedException ignored) {}
                }
            }
            return false;
        }
        return true; // 非 Chrome 浏览器无锁问题
    }

    /** 验证浏览器 cookies 是否真正可提取 */
    private boolean canReadCookies(String browser) {
        try {
            String ytDlp = ytDlpPath != null ? ytDlpPath : ProcessHelper.findYtDlp();
            List<String> cmd = new ArrayList<>();
            cmd.add(ytDlp);
            cmd.add("--cookies-from-browser"); cmd.add(browser);
            cmd.add("--no-warnings"); cmd.add("--no-color");
            if (ProxyConfig.isEnabled()) {
                String proxyArg = ProxyConfig.toCliArgs();
                if (!proxyArg.isEmpty()) {
                    String[] parts = proxyArg.split(" ", 2);
                    if (parts.length == 2) { cmd.add(parts[0]); cmd.add(parts[1]); }
                }
            }
            cmd.add("--version");
            ProcessHelper.CommandResult r = ProcessHelper.executeWithTimeout(cmd, 10);
            return r.exitCode == 0;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * 执行 yt-dlp 命令，遇到 Chrome DB 锁定自动回退到下一个浏览器
     */
    private CommandResult executeCookiesWithRetry(List<String> cmd, long timeoutSeconds)
            throws IOException, InterruptedException {
        CommandResult result = ProcessHelper.executeWithTimeout(cmd, timeoutSeconds);
        if (!result.isSuccess() && util.ChromeCookies.isChromeLockError(result.getStderrText())) {
            // 找下一个可用浏览器
            String fallback = null;
            for (String browser : BROWSER_FALLBACK) {
                if (browser.equalsIgnoreCase(cookiesFromBrowser)) continue;
                if (canReadCookies(browser)) { fallback = browser; break; }
            }
            if (fallback != null) {
                System.out.println("  [!] Chrome 被锁定，自动切换至 " + fallback);
                cookiesFromBrowser = fallback;
                cookiesResolved = true;
                // 用新浏览器重建命令
                List<String> newCmd = rebuildBrowserInCmd(cmd, fallback);
                result = ProcessHelper.executeWithTimeout(newCmd, timeoutSeconds);
            }
        }
        return result;
    }

    /** 替换命令中的 --cookies-from-browser 值 */
    private List<String> rebuildBrowserInCmd(List<String> cmd, String newBrowser) {
        List<String> newCmd = new ArrayList<>(cmd);
        for (int i = 0; i < newCmd.size() - 1; i++) {
            if ("--cookies-from-browser".equals(newCmd.get(i))) {
                newCmd.set(i + 1, newBrowser);
                return newCmd;
            }
        }
        return newCmd;
    }

    private List<String> buildBaseCommand() {
        List<String> cmd = new ArrayList<>();
        cmd.add(ytDlpPath);
        cmd.add("--no-warnings");
        cmd.add("--no-color");

        // 全局代理（对所有操作生效：解析信息、下载、验证等）
        if (ProxyConfig.isEnabled()) {
            String proxyArg = ProxyConfig.toCliArgs();
            if (!proxyArg.isEmpty()) {
                String[] parts = proxyArg.split(" ", 2);
                if (parts.length == 2) {
                    cmd.add(parts[0]);
                    cmd.add(parts[1]);
                }
            }
        }

        // 全局 cookies（对所有操作生效：解析信息、下载等）
        if (cookiesFromBrowser != null && !cookiesFromBrowser.isEmpty()) {
            cmd.add("--cookies-from-browser");
            cmd.add(cookiesFromBrowser);
        } else if (cookiesFile != null && !cookiesFile.isEmpty()) {
            cmd.add("--cookies");
            cmd.add(cookiesFile);
        }

        return cmd;
    }

    /**
     * 解析 yt-dlp JSON 输出到 VideoInfo
     */
    private void parseVideoJson(VideoInfo info, String json) {
        info.setTitle(extractJsonString(json, "title"));
        info.setDescription(extractJsonString(json, "description"));
        info.setDuration(extractJsonLong(json, "duration"));
        info.setThumbnailUrl(extractJsonString(json, "thumbnail"));
        info.setUploader(extractJsonString(json, "uploader"));
        info.setViewCount(extractJsonLong(json, "view_count"));
        info.setLikeCount(extractJsonLong(json, "like_count"));

        // 解析格式
        parseFormats(info, json);
    }

    /**
     * 从 JSON 中解析可用格式
     */
    private void parseFormats(VideoInfo info, String json) {
        // 匹配 "formats": [ ... ]
        int formatsStart = json.indexOf("\"formats\":");
        if (formatsStart < 0) return;

        // 简化解析：提取每个格式对象
        Pattern formatPattern = Pattern.compile("\\{[^}]+\\}");
        String formatsSection = json.substring(formatsStart);

        // 找到 formats 数组结束位置
        int braceCount = 0;
        int arrayStart = formatsSection.indexOf('[');
        if (arrayStart < 0) return;

        int arrayEnd = -1;
        for (int i = arrayStart; i < formatsSection.length(); i++) {
            char c = formatsSection.charAt(i);
            if (c == '[') braceCount++;
            else if (c == ']') {
                braceCount--;
                if (braceCount == 0) {
                    arrayEnd = i;
                    break;
                }
            }
        }
        if (arrayEnd < 0) return;

        String arrayContent = formatsSection.substring(arrayStart + 1, arrayEnd);

        // 匹配每个格式对象
        Matcher m = formatPattern.matcher(arrayContent);
        while (m.find()) {
            String fmtJson = m.group();
            try {
                Format format = parseSingleFormat(fmtJson);
                if (format != null) info.addFormat(format);
            } catch (Exception ignored) {}
        }
    }

    private Format parseSingleFormat(String json) {
        String formatId = extractJsonString(json, "format_id");
        if (formatId == null || formatId.isEmpty()) return null;

        Format fmt = new Format(formatId);
        fmt.setExtension(extractJsonString(json, "ext"));
        fmt.setResolution(extractJsonString(json, "resolution"));
        fmt.setWidth(extractJsonInt(json, "width"));
        fmt.setHeight(extractJsonInt(json, "height"));
        fmt.setFileSize(extractJsonLong(json, "filesize"));
        fmt.setFps((float) extractJsonDouble(json, "fps"));
        fmt.setVideoCodec(extractJsonString(json, "vcodec"));
        fmt.setAudioCodec(extractJsonString(json, "acodec"));
        fmt.setNote(extractJsonString(json, "format_note"));

        // 判断类型
        String vcodec = fmt.getVideoCodec();
        String acodec = fmt.getAudioCodec();
        fmt.setHasVideo(vcodec != null && !vcodec.isEmpty() && !"none".equals(vcodec));
        fmt.setHasAudio(acodec != null && !acodec.isEmpty() && !"none".equals(acodec));

        return fmt;
    }

    // --- JSON 字段提取（简单正则方式，避免引入 JSON 库） ---

    private String extractJsonString(String json, String key) {
        Pattern p = Pattern.compile("\"" + key + "\"\\s*:\\s*\"([^\"]*)\"");
        Matcher m = p.matcher(json);
        if (m.find()) {
            String val = m.group(1);
            if (val.isEmpty() || "null".equals(val)) return null;
            return decodeUnicodeEscapes(val);
        }
        // 尝试 null
        p = Pattern.compile("\"" + key + "\"\\s*:\\s*null");
        m = p.matcher(json);
        if (m.find()) return null;
        return null;
    }

    /** 解码 JSON 中的 \\uXXXX Unicode 转义序列 */
    private static String decodeUnicodeEscapes(String s) {
        if (!s.contains("\\u")) return s;
        StringBuilder sb = new StringBuilder(s.length());
        int i = 0;
        while (i < s.length()) {
            if (i + 5 < s.length() && s.charAt(i) == '\\' && s.charAt(i + 1) == 'u') {
                try {
                    int codePoint = Integer.parseInt(s.substring(i + 2, i + 6), 16);
                    sb.append((char) codePoint);
                    i += 6;
                    continue;
                } catch (NumberFormatException ignored) {}
            }
            sb.append(s.charAt(i));
            i++;
        }
        return sb.toString();
    }

    private long extractJsonLong(String json, String key) {
        Pattern p = Pattern.compile("\"" + key + "\"\\s*:\\s*(-?\\d+)");
        Matcher m = p.matcher(json);
        if (m.find()) return Long.parseLong(m.group(1));
        return 0;
    }

    private int extractJsonInt(String json, String key) {
        Pattern p = Pattern.compile("\"" + key + "\"\\s*:\\s*(-?\\d+)");
        Matcher m = p.matcher(json);
        if (m.find()) return Integer.parseInt(m.group(1));
        return 0;
    }

    private double extractJsonDouble(String json, String key) {
        Pattern p = Pattern.compile("\"" + key + "\"\\s*:\\s*(-?\\d+\\.?\\d*)");
        Matcher m = p.matcher(json);
        if (m.find()) return Double.parseDouble(m.group(1));
        return 0;
    }

    /**
     * 解析进度行
     */
    private DownloadProgress parseProgress(String line) {
        // 格式: downloaded_bytes|total_bytes|speed|eta|percent|status
        String[] parts = line.split("\\|");
        if (parts.length < 5) return null;

        try {
            long downloaded = parseLongSafe(parts[0]);
            long total = parseLongSafe(parts[1]);
            String speed = parts.length > 2 ? parts[2] : "";
            String eta = parts.length > 3 ? parts[3] : "";
            String percent = parts.length > 4 ? parts[4] : "";
            String status = parts.length > 5 ? parts[5] : "";

            return new DownloadProgress(downloaded, total, speed, eta, percent, status);
        } catch (Exception e) {
            return null;
        }
    }

    private long parseLongSafe(String s) {
        if (s == null || s.isEmpty() || "NA".equals(s) || "Unknown".equals(s)) return 0;
        try {
            return (long) Double.parseDouble(s);
        } catch (NumberFormatException e) {
            return 0;
        }
    }

    // ==================== 内部类 ====================

    /**
     * 下载进度
     */
    public static class DownloadProgress {
        public final long downloadedBytes;
        public final long totalBytes;
        public final String speed;
        public final String eta;
        public final String percent;
        public final String status;

        public DownloadProgress(long downloadedBytes, long totalBytes,
                                 String speed, String eta, String percent, String status) {
            this.downloadedBytes = downloadedBytes;
            this.totalBytes = totalBytes;
            this.speed = speed;
            this.eta = eta;
            this.percent = percent;
            this.status = status;
        }

        public double getPercentValue() {
            if (totalBytes > 0) return downloadedBytes * 100.0 / totalBytes;
            return 0;
        }

        @Override
        public String toString() {
            double pct = getPercentValue();
            int barLen = 30;
            int filled = (int) (pct / 100 * barLen);

            StringBuilder bar = new StringBuilder("[");
            for (int i = 0; i < barLen; i++) {
                if (i < filled) bar.append("=");
                else if (i == filled) bar.append(">");
                else bar.append(" ");
            }
            bar.append("]");

            String displaySpeed = (speed != null && !speed.isEmpty() && !"NA".equals(speed)) ? speed : "?";
            String displayEta = (eta != null && !eta.isEmpty() && !"NA".equals(eta)) ? eta : "...";
            return String.format("%s %s | %s | ETA: %s",
                    bar.toString(), percent, displaySpeed, displayEta);
        }
    }
}
