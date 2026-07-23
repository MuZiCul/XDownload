import downloader.YtDlpDownloader;
import model.DownloadConfig;
import model.VideoInfo;
import ui.ConsoleUI;
import util.Bootstrap;
import util.ProcessHelper;

import java.util.List;

/**
 * XDownload - X视频下载工具
 * <p>
 * 基于开源项目 yt-dlp (https://github.com/yt-dlp/yt-dlp)
 * 支持 1000+ 视频网站的视频/音频下载
 * <p>
 * 用法:
 *   java Main                   交互模式
 *   java Main <URL>             下载最佳质量
 *   java Main <URL> -f 18       指定格式
 *   java Main <URL> -o ./videos 指定输出目录
 *   java Main <URL> -x          仅提取音频
 *   java Main <URL> --info      仅查看信息不下载
 *   java Main -b urls.txt       批量下载
 */
public class Main {

    private static final String VERSION = "1.0.0";

    public static void main(String[] args) {
        // 编码设置，避免中文乱码
        System.setProperty("file.encoding", "UTF-8");

        if (args.length == 0) {
            // 交互模式
            new ConsoleUI().start();
            return;
        }

        // 命令行模式
        try {
            handleCommandLine(args);
        } catch (Exception e) {
            System.err.println("错误: " + e.getMessage());
            System.exit(1);
        }
    }

    private static void handleCommandLine(String[] args) throws Exception {
        // 帮助
        if (args[0].equals("-h") || args[0].equals("--help")) {
            printHelp();
            return;
        }

        // 版本
        if (args[0].equals("-v") || args[0].equals("--version")) {
            System.out.println("XDownload v" + VERSION);
            return;
        }

        // 批量下载
        if (args[0].equals("-b") || args[0].equals("--batch")) {
            if (args.length < 2) {
                System.err.println("用法: java Main -b <URL列表文件>");
                return;
            }
            // 转到交互模式的批量下载
            new ConsoleUI().start();
            return;
        }

        // 单URL下载
        String url = args[0];
        DownloadConfig config = new DownloadConfig(url);
        boolean infoOnly = false;

        for (int i = 1; i < args.length; i++) {
            switch (args[i]) {
                case "-f":
                case "--format":
                    if (i + 1 < args.length) config.setFormatId(args[++i]);
                    break;
                case "-o":
                case "--output":
                    if (i + 1 < args.length) config.setOutputDir(args[++i]);
                    break;
                case "-x":
                case "--extract-audio":
                    config.setExtractAudio(true);
                    break;
                case "-t":
                case "--template":
                    if (i + 1 < args.length) config.setOutputTemplate(args[++i]);
                    break;
                case "-p":
                case "--proxy":
                    if (i + 1 < args.length) config.setProxy(args[++i]);
                    break;
                case "-c":
                case "--cookies":
                    if (i + 1 < args.length) config.setCookiesFile(args[++i]);
                    break;
                case "-cb":
                case "--cookies-from-browser":
                    if (i + 1 < args.length) config.setCookiesFromBrowser(args[++i]);
                    break;
                case "--info":
                    infoOnly = true;
                    break;
                case "-r":
                case "--retries":
                    if (i + 1 < args.length) config.setRetries(Integer.parseInt(args[++i]));
                    break;
                case "--max-height":
                    if (i + 1 < args.length) config.setMaxHeight(Integer.parseInt(args[++i]));
                    break;
                case "--embed-subs":
                    config.setEmbedSubtitles(true);
                    break;
                case "--embed-thumb":
                    config.setEmbedThumbnail(true);
                    break;
                default:
                    System.err.println("未知参数: " + args[i]);
                    printHelp();
                    return;
            }
        }

        YtDlpDownloader downloader = new YtDlpDownloader();

        // 将命令行 cookies/proxy 设置同步到 downloader（fetchVideoInfo 依赖这些全局设置）
        if (config.getCookiesFromBrowser() != null && !config.getCookiesFromBrowser().isEmpty()) {
            downloader.setCookiesFromBrowser(config.getCookiesFromBrowser());
        } else if (config.getCookiesFile() != null && !config.getCookiesFile().isEmpty()) {
            downloader.setCookiesFile(config.getCookiesFile());
        }

        // 确保 yt-dlp 可用（命令行模式也需要自动下载）
        try {
            Bootstrap.ensureYtDlp();
        } catch (Exception e) {
            System.err.println("yt-dlp 不可用: " + e.getMessage());
            System.exit(1);
        }

        if (infoOnly) {
            // 仅查看信息
            System.out.println("正在获取视频信息...");
            VideoInfo info = downloader.fetchVideoInfo(url);
            System.out.println(info);
            return;
        }

        // 下载
        System.out.println("开始下载: " + url);
        boolean success = downloader.download(config, progress -> {
            System.out.print("\r  " + progress.toString());
        });
        System.out.println();

        if (success) {
            System.out.println("下载完成!");
        } else {
            System.err.println("下载失败");
            System.exit(1);
        }
    }

    private static void printHelp() {
        System.out.println("XDownload v" + VERSION + " - X视频下载工具");
        System.out.println();
        System.out.println("用法:");
        System.out.println("  java Main                    交互模式");
        System.out.println("  java Main <URL>              下载最佳质量");
        System.out.println();
        System.out.println("参数:");
        System.out.println("  -f, --format <id>      指定格式 (如: 18, 22, best, bestvideo+bestaudio)");
        System.out.println("  -o, --output <dir>     输出目录 (默认: downloads)");
        System.out.println("  -t, --template <tpl>   文件名模板 (默认: %(title)s.%(ext)s)");
        System.out.println("  -x, --extract-audio    仅提取音频 (MP3)");
        System.out.println("  -p, --proxy <url>      代理地址");
        System.out.println("  -c, --cookies <file>   Cookies文件路径 (Netscape格式)");
        System.out.println("  -cb, --cookies-from-browser <浏览器>  从浏览器读Cookies (chrome/firefox/edge/brave/opera)");
        System.out.println("  -r, --retries <n>      重试次数 (默认: 5)");
        System.out.println("  --max-height <n>       最大分辨率限制");
        System.out.println("  --embed-subs           嵌入字幕");
        System.out.println("  --embed-thumb          嵌入缩略图");
        System.out.println("  --info                 仅查看信息不下载");
        System.out.println("  -h, --help             显示帮助");
        System.out.println("  -v, --version          显示版本");
        System.out.println();
        System.out.println("示例:");
        System.out.println("  java Main https://www.youtube.com/watch?v=xxx");
        System.out.println("  java Main https://www.bilibili.com/video/BV1xx -x");
        System.out.println("  java Main https://example.com/video -o ./my_videos --info");
        System.out.println();
        System.out.println("依赖:");
        System.out.println("  yt-dlp (必须): https://github.com/yt-dlp/yt-dlp");
        System.out.println("  ffmpeg (推荐): https://ffmpeg.org");
    }
}