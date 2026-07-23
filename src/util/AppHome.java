package util;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

/**
 * 统一解析应用根目录 —— 兼容 IDE 运行、命令行、jpackage 打包三种场景
 */
public class AppHome {

    /** 应用根目录（bin/、config/、downloads/ 的父目录） */
    public static final Path ROOT = resolve();

    /** bin/ 目录 */
    public static final Path BIN = ROOT.resolve("bin");

    /** config/ 目录 */
    public static final Path CONFIG = ROOT.resolve("config");

    /** downloads/ 目录 */
    public static final Path DOWNLOADS = ROOT.resolve("downloads");

    private static Path resolve() {
        try {
            Path start = Paths.get(AppHome.class.getProtectionDomain()
                    .getCodeSource().getLocation().toURI());
            // JAR 文件：往上到 JAR 所在目录
            if (Files.isRegularFile(start)) {
                start = start.getParent();
            }
            // jpackage: JAR 在 app/ 子目录，父目录即安装根
            if (start != null && "app".equals(start.getFileName().toString())) {
                Path parent = start.getParent();
                if (parent != null) return parent;
            }
            // IDE / 命令行：从 classes 目录向上找含 bin/ 的目录
            Path current = start;
            while (current != null) {
                if (Files.exists(current.resolve("bin/yt-dlp.exe"))
                        || Files.exists(current.resolve("bin/yt-dlp"))) {
                    return current;
                }
                current = current.getParent();
            }
            return start != null ? start : Paths.get(System.getProperty("user.dir"));
        } catch (Exception e) {
            return Paths.get(System.getProperty("user.dir")).toAbsolutePath();
        }
    }
}
