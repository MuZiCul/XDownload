package util;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;

/**
 * Chrome Cookies 工具 —— 绕过 Chrome 数据库锁定问题
 */
public class ChromeCookies {

    /** Chrome Cookie 数据库在 Windows 上的默认路径 */
    private static final Path CHROME_COOKIES_DB = Paths.get(
            System.getenv("LOCALAPPDATA"),
            "Google", "Chrome", "User Data", "Default", "Network", "Cookies");

    /** 项目内备份目录 */
    private static final Path BACKUP_DIR = AppHome.CONFIG;

    /** 备份文件名 */
    private static final String BACKUP_NAME = "chrome_cookies_backup.db";

    /**
     * 尝试手动复制 Chrome Cookies 数据库到项目 config 目录
     * <p>
     * Java NIO 在 Windows 上可以读取被 Chrome 以共享读模式打开的文件，
     * 而 Python shutil.copy2 在某些情况下会失败。
     *
     * @return 备份文件路径，失败返回 null
     */
    public static Path backupCookiesDb() {
        if (!Files.exists(CHROME_COOKIES_DB)) {
            return null;
        }

        try {
            Files.createDirectories(BACKUP_DIR);
            Path backup = BACKUP_DIR.resolve(BACKUP_NAME);
            Files.copy(CHROME_COOKIES_DB, backup, StandardCopyOption.REPLACE_EXISTING);
            return backup;
        } catch (IOException e) {
            return null;
        }
    }

    /**
     * 检查备份是否可用且未过期（1小时内）
     */
    public static boolean isBackupValid() {
        Path backup = BACKUP_DIR.resolve(BACKUP_NAME);
        if (!Files.exists(backup)) return false;
        try {
            long age = System.currentTimeMillis() - Files.getLastModifiedTime(backup).toMillis();
            return age < 3600_000; // 1 小时
        } catch (IOException e) {
            return false;
        }
    }

    /**
     * 获取备份文件路径（可能不存在）
     */
    public static Path getBackupPath() {
        return BACKUP_DIR.resolve(BACKUP_NAME);
    }

    /**
     * 判断错误是否是由 Chrome 数据库锁定导致的
     */
    public static boolean isChromeLockError(String stderr) {
        String lower = stderr.toLowerCase();
        return lower.contains("could not copy") && lower.contains("chrome")
                || lower.contains("could not copy") && lower.contains("cookie database");
    }
}
