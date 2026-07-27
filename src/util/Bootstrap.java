package util;

import java.io.*;
import java.net.HttpURLConnection;
import java.net.Proxy;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.List;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

/**
 * 环境引导工具 —— 首次运行时自动下载 yt-dlp / ffmpeg 到项目 bin/ 目录
 * <p>
 * 无需用户手动安装任何依赖，开箱即用。
 */
public class Bootstrap {

    /** 项目根目录 */
    public static final Path PROJECT_ROOT = AppHome.ROOT;
    /** 项目内 bin 目录路径 */
    public static final Path BIN_DIR = AppHome.BIN;

    // ==== 下载源 URL（按优先级排列，自动回退） ====

    /** yt-dlp 下载源列表：直连 → 国内镜像 */
    private static final String[] YTDLP_URLS = {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
        "https://mirror.ghproxy.com/https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
    };

    /** ffmpeg 下载源列表 */
    private static final String[] FFMPEG_URLS = {
        "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        "https://mirror.ghproxy.com/https://github.com/GyanD/codexffmpeg/releases/download/2024-12-26-git-5a3afe06e2/ffmpeg-2024-12-26-git-5a3afe06e2-essentials_build.zip",
    };

    // ==== 公开方法 ====

    /**
     * 确保 yt-dlp 可用；若缺失或损坏则自动下载。
     * @return yt-dlp 可执行文件路径
     */
    public static String ensureYtDlp() throws IOException {
        // 1. 先检查项目 bin 目录
        Path binYtdlp = BIN_DIR.resolve("yt-dlp.exe");
        if (Files.exists(binYtdlp)) {
            if (validateYtDlp(binYtdlp)) {
                return binYtdlp.toAbsolutePath().toString();
            }
            // 二进制损坏，删除后重新下载
            System.out.println("  [!] bin/yt-dlp.exe 已损坏，删除并重新下载...");
            try { Files.delete(binYtdlp); } catch (Exception ignored) {}
        }

        // 2. 检查系统 PATH
        if (ProcessHelper.isYtDlpAvailable()) {
            return ProcessHelper.findYtDlp();
        }

        // 3. 自动下载
        System.out.println("  [...] 首次使用，正在自动下载 yt-dlp ...");
        System.out.println("     (约 15MB，请耐心等待)");
        if (ProxyConfig.isEnabled()) {
            System.out.println("     使用代理: " + ProxyConfig.getProxyString());
        }

        Files.createDirectories(BIN_DIR);

        // 多源重试
        IOException lastError = null;
        for (int i = 0; i < YTDLP_URLS.length; i++) {
            String url = YTDLP_URLS[i];
            if (i > 0) System.out.println("  [!] 切换至镜像源 " + (i + 1) + " ...");
            try {
                downloadFile(url, binYtdlp, "yt-dlp");
                binYtdlp.toFile().setExecutable(true);

                // 下载后验证二进制完整性
                if (!validateYtDlp(binYtdlp)) {
                    throw new IOException("下载的 yt-dlp 二进制无法运行（可能不完整）");
                }

                System.out.println("  [+] yt-dlp 下载完成并验证通过: " + binYtdlp);
                return binYtdlp.toAbsolutePath().toString();
            } catch (IOException e) {
                lastError = e;
                System.out.println("  [!] 源 " + (i + 1) + " 失败: " + e.getMessage());
                // 清除不完整的文件
                try { Files.deleteIfExists(binYtdlp); } catch (Exception ignored) {}
            }
        }

        throw new IOException(
            "yt-dlp 自动下载失败（已尝试 " + YTDLP_URLS.length + " 个源），请手动安装:\n" +
            "  winget install yt-dlp.yt-dlp\n" +
            "  或访问 https://github.com/yt-dlp/yt-dlp/releases\n" +
            "  提示: 可使用 -Dhttp.proxyHost=127.0.0.1 -Dhttp.proxyPort=7890 设置代理", lastError);
    }

    /**
     * 尝试确保 ffmpeg 可用；若缺失则询问用户是否自动下载（因为体积较大 ~80MB）。
     * @param autoConfirm 是否自动确认下载（命令行模式为 true，交互模式为 false）
     * @return ffmpeg 路径，若不可用返回 null（不影响核心功能）
     */
    public static String ensureFfmpeg(boolean autoConfirm) throws IOException {
        // 1. 检查项目 bin 目录
        Path binFfmpeg = BIN_DIR.resolve("ffmpeg.exe");
        if (Files.exists(binFfmpeg)) {
            return binFfmpeg.toAbsolutePath().toString();
        }

        // 2. 检查系统 PATH
        if (ProcessHelper.isFfmpegAvailable()) {
            return "ffmpeg";
        }

        // 3. ffmpeg 非必须，仅提示
        if (!autoConfirm) {
            return null; // 交互模式下由 GUI StartupWizard 处理
        }

        // 命令行模式：静默尝试下载
        System.out.println("  [...] 正在自动下载 ffmpeg (约 80MB，仅首次需要)...");
        if (ProxyConfig.isEnabled()) {
            System.out.println("     使用代理: " + ProxyConfig.getProxyString());
        }

        IOException lastError = null;
        for (int i = 0; i < FFMPEG_URLS.length; i++) {
            String url = FFMPEG_URLS[i];
            if (i > 0) System.out.println("  [!] 切换至镜像源 " + (i + 1) + " ...");
            try {
                Files.createDirectories(BIN_DIR);
                Path zipPath = BIN_DIR.resolve("ffmpeg-temp.zip");
                downloadFile(url, zipPath, "ffmpeg");
                extractFfmpeg(zipPath, BIN_DIR);
                Files.deleteIfExists(zipPath);
                System.out.println("  [+] ffmpeg 下载完成");
                return BIN_DIR.resolve("ffmpeg.exe").toAbsolutePath().toString();
            } catch (IOException e) {
                lastError = e;
                System.out.println("  [!] 源 " + (i + 1) + " 失败: " + e.getMessage());
            }
        }

        System.out.println("  [!] ffmpeg 自动下载失败（不影响基本下载功能）");
        System.out.println("    手动安装: winget install ffmpeg  或  https://ffmpeg.org");
        return null;
    }

    // ==== 文件下载 ====

    /**
     * 下载文件到指定路径，带进度显示（自动走代理）
     */
    private static void downloadFile(String urlStr, Path dest, String label) throws IOException {
        URL url = new URL(urlStr);
        Proxy proxy = ProxyConfig.toJavaProxy();
        HttpURLConnection conn = (HttpURLConnection) url.openConnection(proxy);
        conn.setRequestProperty("User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) XDownload/1.0");
        conn.setInstanceFollowRedirects(true);
        conn.setConnectTimeout(ProxyConfig.isEnabled() ? 30000 : 15000);
        conn.setReadTimeout(120000);

        int responseCode = conn.getResponseCode();
        // 处理重定向
        if (responseCode == HttpURLConnection.HTTP_MOVED_TEMP
                || responseCode == HttpURLConnection.HTTP_MOVED_PERM
                || responseCode == HttpURLConnection.HTTP_SEE_OTHER) {
            String redirectUrl = conn.getHeaderField("Location");
            conn.disconnect();
            downloadFile(redirectUrl, dest, label);
            return;
        }

        long totalSize = conn.getContentLengthLong();
        String sizeStr = totalSize > 0 ? formatSize(totalSize) : "未知大小";
        System.out.println("    下载 " + label + " (" + sizeStr + ") ...");

        try (InputStream in = new BufferedInputStream(conn.getInputStream());
             OutputStream out = new BufferedOutputStream(Files.newOutputStream(dest))) {

            byte[] buffer = new byte[8192];
            long downloaded = 0;
            int read;
            int lastPct = -1;

            while ((read = in.read(buffer)) != -1) {
                out.write(buffer, 0, read);
                downloaded += read;

                if (totalSize > 0) {
                    int pct = (int) (downloaded * 100 / totalSize);
                    if (pct != lastPct) {
                        lastPct = pct;
                        printProgress(pct, totalSize, downloaded);
                    }
                } else {
                    // 仅显示已下载大小
                    if (downloaded % (1024 * 1024) < 8192) {
                        System.out.print("\r    已下载: " + formatSize(downloaded) + "     ");
                    }
                }
            }
            System.out.println();  // 换行
        } finally {
            conn.disconnect();
        }
    }

    /**
     * 从 ffmpeg zip 包中提取 ffmpeg.exe, ffprobe.exe, ffplay.exe 到 bin 目录
     */
    private static void extractFfmpeg(Path zipPath, Path destDir) throws IOException {
        System.out.println("    正在解压 ffmpeg ...");
        try (ZipInputStream zis = new ZipInputStream(
                new BufferedInputStream(Files.newInputStream(zipPath)))) {

            ZipEntry entry;
            while ((entry = zis.getNextEntry()) != null) {
                String name = entry.getName();
                // 只提取 bin 目录下的 exe 文件
                if (entry.isDirectory()) continue;

                String lowerName = name.toLowerCase();
                boolean isTarget = lowerName.endsWith("ffmpeg.exe")
                        || lowerName.endsWith("ffprobe.exe")
                        || lowerName.endsWith("ffplay.exe");

                if (!isTarget) continue;

                // 提取文件名
                String fileName = name.substring(name.lastIndexOf('/') + 1);
                Path target = destDir.resolve(fileName);
                System.out.println("    [+] 解压: " + fileName);

                Files.copy(zis, target, StandardCopyOption.REPLACE_EXISTING);
                target.toFile().setExecutable(true);
            }
        }
    }

    // ==== 进度条 ====

    private static void printProgress(int percent, long total, long downloaded) {
        int barLen = 25;
        int filled = percent * barLen / 100;

        StringBuilder bar = new StringBuilder("\r    [");
        for (int i = 0; i < barLen; i++) {
            if (i < filled) bar.append("=");
            else if (i == filled) bar.append(">");
            else bar.append(" ");
        }
        bar.append("] ");
        bar.append(String.format("%3d%%", percent));
        bar.append("  ");
        bar.append(formatSize(downloaded));
        bar.append(" / ");
        bar.append(formatSize(total));

        System.out.print(bar.toString());
    }

    public static String formatSize(long bytes) {
        if (bytes < 1024) return bytes + "B";
        if (bytes < 1024 * 1024) return String.format("%.1fKB", bytes / 1024.0);
        if (bytes < 1024 * 1024 * 1024) return String.format("%.1fMB", bytes / (1024.0 * 1024));
        return String.format("%.2fGB", bytes / (1024.0 * 1024 * 1024));
    }

    // ==== 二进制验证 ====

    /**
     * 验证 yt-dlp 二进制是否真正可用（跑 --version 检测）
     * @return true 表示二进制完整可用
     */
    private static boolean validateYtDlp(Path exePath) {
        try {
            ProcessHelper.CommandResult result = ProcessHelper.executeWithTimeout(
                    List.of(exePath.toAbsolutePath().toString(), "--version"), 10);
            return result.exitCode == 0;
        } catch (Exception e) {
            return false;
        }
    }
}
