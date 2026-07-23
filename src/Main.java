import ui.gui.XDownloadApp;
import util.Version;

/**
 * XDownload - X视频下载工具
 * <p>
 * 基于 yt-dlp (https://github.com/yt-dlp/yt-dlp)
 * 支持 1000+ 视频网站的视频/音频下载
 */
public class Main {

    public static void main(String[] args) {
        // --help / --version
        if (args.length > 0 && (args[0].equals("-h") || args[0].equals("--help"))) {
            printHelp();
            return;
        }
        if (args.length > 0 && (args[0].equals("-v") || args[0].equals("--version"))) {
            System.out.println("XDownload v" + Version.CURRENT);
            return;
        }

        // 启动 GUI
        XDownloadApp.launch();
    }

    private static void printHelp() {
        System.out.println("XDownload v" + Version.CURRENT + " - X视频下载工具");
        System.out.println("基于 yt-dlp | 支持 X/Twitter, YouTube 等 1000+ 网站");
        System.out.println();
        System.out.println("用法:");
        System.out.println("  java Main                启动 GUI 图形界面");
        System.out.println("  java Main -h, --help     显示帮助");
        System.out.println("  java Main -v, --version  显示版本");
        System.out.println();
        System.out.println("构建便携版:");
        System.out.println("  build.bat");
        System.out.println();
        System.out.println("依赖:");
        System.out.println("  yt-dlp (必须): https://github.com/yt-dlp/yt-dlp");
        System.out.println("  ffmpeg (推荐): https://ffmpeg.org");
    }
}
