package util;

import java.io.BufferedReader;
import java.io.File;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.function.Consumer;

/**
 * 进程调用工具，封装 yt-dlp 命令行调用
 */
public class ProcessHelper {

    /**
     * 执行命令并返回结果
     */
    public static CommandResult execute(List<String> command) throws IOException, InterruptedException {
        return execute(command, null, null);
    }

    /**
     * 执行命令，实时回调每行输出（不暴露 Process 引用）
     */
    public static CommandResult execute(List<String> command,
                                         Consumer<String> stdoutCallback,
                                         Consumer<String> stderrCallback)
            throws IOException, InterruptedException {
        return execute(command, stdoutCallback, stderrCallback, null);
    }

    /**
     * 执行命令，实时回调每行输出，可选暴露 Process 引用（供取消下载）
     */
    public static CommandResult execute(List<String> command,
                                         Consumer<String> stdoutCallback,
                                         Consumer<String> stderrCallback,
                                         java.util.concurrent.atomic.AtomicReference<Process> processRef)
            throws IOException, InterruptedException {

        ProcessBuilder pb = new ProcessBuilder(command);
        pb.redirectErrorStream(false);

        Process process = pb.start();
        if (processRef != null) processRef.set(process);
        List<String> stdout = new ArrayList<>();
        List<String> stderr = new ArrayList<>();

        // 读取 stdout
        Thread stdoutThread = new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    stdout.add(line);
                    if (stdoutCallback != null) stdoutCallback.accept(line);
                }
            } catch (IOException ignored) {}
        }, "stdout-reader");

        // 读取 stderr
        Thread stderrThread = new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getErrorStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    stderr.add(line);
                    if (stderrCallback != null) stderrCallback.accept(line);
                }
            } catch (IOException ignored) {}
        }, "stderr-reader");

        stdoutThread.start();
        stderrThread.start();

        int exitCode = process.waitFor();
        stdoutThread.join(5000);
        stderrThread.join(5000);

        return new CommandResult(exitCode, stdout, stderr);
    }

    /**
     * 执行命令，带超时
     */
    public static CommandResult executeWithTimeout(List<String> command, long timeoutSeconds)
            throws IOException, InterruptedException {
        ProcessBuilder pb = new ProcessBuilder(command);
        pb.redirectErrorStream(false);

        Process process = pb.start();
        List<String> stdout = new ArrayList<>();
        List<String> stderr = new ArrayList<>();

        Thread stdoutThread = new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) stdout.add(line);
            } catch (IOException ignored) {}
        });
        Thread stderrThread = new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getErrorStream()))) {
                String line;
                while ((line = reader.readLine()) != null) stderr.add(line);
            } catch (IOException ignored) {}
        });

        stdoutThread.start();
        stderrThread.start();

        boolean finished = process.waitFor(timeoutSeconds, TimeUnit.SECONDS);
        if (!finished) {
            process.destroyForcibly();
            throw new IOException("命令执行超时（" + timeoutSeconds + "秒）");
        }

        // 等待读取线程完成（进程已退出，线程很快结束）
        stdoutThread.join(3000);
        stderrThread.join(3000);

        return new CommandResult(process.exitValue(), stdout, stderr);
    }

    /** 项目 bin 目录 */
    public static final Path BIN_DIR = AppHome.BIN;

    /**
     * 查找 yt-dlp 可执行文件（优先项目 bin 目录）
     */
    public static String findYtDlp() {
        // 1. 优先检查项目 bin 目录
        Path binYtdlp = BIN_DIR.resolve("yt-dlp.exe");
        if (Files.exists(binYtdlp) && isExecutableFile(binYtdlp)) {
            return binYtdlp.toAbsolutePath().toString();
        }

        // 2. 检查当前目录
        String[] localNames = {"yt-dlp.exe", "yt-dlp", "yt-dlp_x86.exe"};
        for (String name : localNames) {
            File f = new File(name);
            if (f.exists() && isExecutableFile(f.toPath())) return f.getAbsolutePath();
        }

        // 3. 检查系统 PATH
        String[] pathNames = {"yt-dlp.exe", "yt-dlp"};
        for (String name : pathNames) {
            try {
                CommandResult result = execute(List.of(
                        isWindows() ? "where" : "which", name));
                if (result.exitCode == 0 && !result.stdout.isEmpty()) {
                    return result.stdout.get(0).trim();
                }
            } catch (Exception ignored) {}
        }

        return "yt-dlp"; // 默认，让系统自己找
    }

    /**
     * 检查文件是否可执行（Windows 兼容）
     * 在 Windows 上 Files.isExecutable() 不可靠，只要是 .exe/.bat 文件即可执行
     */
    private static boolean isExecutableFile(Path path) {
        if (Files.exists(path)) {
            if (isWindows()) {
                // Windows: 只要是存在的文件就认为"可执行"
                // Files.isExecutable() 在 Windows 上不可靠（依赖 DACL，映射不准）
                return true;
            }
            return Files.isExecutable(path);
        }
        return false;
    }

    /**
     * 查找 ffmpeg 可执行文件（优先项目 bin 目录）
     */
    public static String findFfmpeg() {
        // 1. 优先检查项目 bin 目录
        Path binFfmpeg = BIN_DIR.resolve("ffmpeg.exe");
        if (Files.exists(binFfmpeg) && isExecutableFile(binFfmpeg)) {
            return binFfmpeg.toAbsolutePath().toString();
        }

        // 2. 检查系统 PATH
        try {
            CommandResult result = execute(List.of(
                    isWindows() ? "where" : "which", "ffmpeg"));
            if (result.exitCode == 0 && !result.stdout.isEmpty()) {
                return result.stdout.get(0).trim();
            }
        } catch (Exception ignored) {}

        return "ffmpeg"; // 默认
    }

    /**
     * 检查 yt-dlp 是否可用
     */
    public static boolean isYtDlpAvailable() {
        try {
            CommandResult result = execute(List.of(findYtDlp(), "--version"));
            return result.exitCode == 0;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * 检查 ffmpeg 是否可用（优先查项目 bin 目录）
     */
    public static boolean isFfmpegAvailable() {
        try {
            // 先快速检查文件是否存在（避免每次启动进程）
            Path binFfmpeg = BIN_DIR.resolve("ffmpeg.exe");
            if (Files.exists(binFfmpeg) && isExecutableFile(binFfmpeg)) {
                return true;
            }
            CommandResult result = execute(List.of(findFfmpeg(), "-version"));
            return result.exitCode == 0;
        } catch (Exception e) {
            return false;
        }
    }

    public static boolean isWindows() {
        return System.getProperty("os.name").toLowerCase().contains("win");
    }

    /**
     * 验证指定浏览器的 cookies 是否可提取（本地操作，不发网络请求）
     * <p>
     * 通过解析 yt-dlp 的输出来确认 cookies 真正被提取到了，而不是仅检查数据库能否打开。
     * @return 验证结果，含提取到的 cookie 数量
     */
    public static CookiesValidationResult validateCookiesFromBrowser(String browser) {
        try {
            String ytDlp = findYtDlp();
            // 构建命令，带上代理（如果有的话）
            List<String> cmd = new ArrayList<>();
            cmd.add(ytDlp);
            cmd.add("--cookies-from-browser"); cmd.add(browser);
            cmd.add("--no-warnings");
            cmd.add("--no-color");
            // 代理
            if (ProxyConfig.isEnabled()) {
                String proxyArg = ProxyConfig.toCliArgs();
                if (!proxyArg.isEmpty()) {
                    String[] parts = proxyArg.split(" ", 2);
                    if (parts.length == 2) {
                        cmd.add(parts[0]); cmd.add(parts[1]);
                    }
                }
            }
            cmd.add("--version");

            CommandResult result = executeWithTimeout(cmd, 10);

            String stderr = result.getStderrText();
            String combined = (result.getStdoutText() + "\n" + stderr).toLowerCase();
            String lower = stderr.toLowerCase();

            if (result.exitCode != 0) {
                // Chrome 数据库被锁定（浏览器正在运行）
                if (lower.contains("could not copy") || (lower.contains("copy") && lower.contains("database"))) {
                    return new CookiesValidationResult(false,
                            "[-] " + browser + " 正在运行，Cookie 数据库被锁定\n"
                                    + "     → 请完全关闭 " + browser + " 浏览器后重试", 0);
                }
                if (lower.contains("could not find") || lower.contains("not found")
                        || lower.contains("no such file") || lower.contains("does not exist")) {
                    return new CookiesValidationResult(false,
                            "[-] 未找到 " + browser + " Cookie 数据库（浏览器未安装或从未使用）", 0);
                }
                if (lower.contains("permission") || lower.contains("denied")
                        || lower.contains("locked") || lower.contains("access")) {
                    return new CookiesValidationResult(false,
                            "[-] " + browser + " Cookie 数据库被锁定（请关闭浏览器后重试）", 0);
                }
                if (lower.contains("keyring") || lower.contains("decrypt") || lower.contains("encrypt")) {
                    return new CookiesValidationResult(false,
                            "[-] " + browser + " Cookie 解密失败（尝试关闭浏览器或管理员运行）", 0);
                }
                return new CookiesValidationResult(false,
                        "[-] " + browser + " Cookies 读取失败: " + stderr.trim(), 0);
            }

            // 成功了 —— 解析实际提取到的 cookie 数量
            // yt-dlp 格式: "[Cookies] Extracted 247 cookies from chrome"
            int count = extractCookieCount(combined);
            if (count > 0) {
                return new CookiesValidationResult(true,
                        "[+] " + browser + " Cookies 就绪（提取 " + count + " 条）", count);
            } else if (combined.contains("extracted") && combined.contains("cookie")) {
                // 明确输出了提取信息但数量为 0
                return new CookiesValidationResult(true,
                        "[!] " + browser + " 提取到 0 条 Cookie（浏览器未登录任何网站）", 0);
            } else {
                // 没报错，但也没有输出提取信息（某些 yt-dlp 版本 --version 不触发提取日志）
                return new CookiesValidationResult(true,
                        "[+] " + browser + " Cookies 已加载", 0);
            }
        } catch (Exception e) {
            String msg = e.getMessage();
            if (msg != null && msg.contains("超时")) {
                return new CookiesValidationResult(false,
                        "[-] " + browser + " 验证超时（请关闭浏览器后重试，或检查系统负载）", 0);
            }
            return new CookiesValidationResult(false,
                    "[-] Cookies 验证异常: " + (msg != null ? msg : e.getClass().getSimpleName()), 0);
        }
    }

    /**
     * 从 yt-dlp 输出中解析提取的 cookie 数量
     * 典型格式: "[Cookies] Extracted 247 cookies from chrome"
     */
    private static int extractCookieCount(String text) {
        java.util.regex.Pattern p = java.util.regex.Pattern.compile(
                "(?i)extracted\\s+(\\d+)\\s+cookies?");
        java.util.regex.Matcher m = p.matcher(text);
        if (m.find()) {
            try {
                return Integer.parseInt(m.group(1));
            } catch (NumberFormatException ignored) {}
        }
        return -1;
    }

    /**
     * Cookies 验证结果
     */
    public static class CookiesValidationResult {
        public final boolean success;
        public final String message;
        public final int cookieCount;

        public CookiesValidationResult(boolean success, String message, int cookieCount) {
            this.success = success;
            this.message = message;
            this.cookieCount = cookieCount;
        }
    }

    /**
     * 命令执行结果
     */
    public static class CommandResult {
        public final int exitCode;
        public final List<String> stdout;
        public final List<String> stderr;

        public CommandResult(int exitCode, List<String> stdout, List<String> stderr) {
            this.exitCode = exitCode;
            this.stdout = stdout;
            this.stderr = stderr;
        }

        public boolean isSuccess() { return exitCode == 0; }

        public String getStdoutText() { return String.join("\n", stdout); }

        public String getStderrText() { return String.join("\n", stderr); }
    }
}
