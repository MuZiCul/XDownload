package util;

import java.io.IOException;
import java.net.HttpURLConnection;
import java.net.InetSocketAddress;
import java.net.Proxy;
import java.net.URL;

/**
 * 全局代理配置 —— 控制 Bootstrap 下载和 yt-dlp 网络请求的代理
 * <p>
 * 优先级：主动设置 > 系统属性 (http.proxyHost / https.proxyHost) > 环境变量 (HTTP_PROXY / HTTPS_PROXY)
 */
public class ProxyConfig {

    private static String proxyHost;
    private static int proxyPort = -1;
    private static Proxy.Type proxyType = Proxy.Type.HTTP;
    private static boolean enabled = false;

    static {
        // 自动从系统属性读取
        loadFromSystemProperties();
    }

    // ==================== Getter / Setter ====================

    public static String getProxyHost() { return proxyHost; }
    public static int getProxyPort() { return proxyPort; }
    public static Proxy.Type getProxyType() { return proxyType; }
    public static boolean isEnabled() { return enabled && proxyHost != null && !proxyHost.isEmpty(); }

    /**
     * 手动设置代理
     */
    public static void setProxy(String host, int port) {
        setProxy(host, port, Proxy.Type.HTTP);
    }

    public static void setProxy(String host, int port, Proxy.Type type) {
        proxyHost = host;
        proxyPort = port;
        proxyType = type;
        enabled = true;
    }

    /**
     * 禁用代理
     */
    public static void disable() {
        enabled = false;
    }

    /**
     * 获取 java.net.Proxy 对象（供 HttpURLConnection 使用）
     */
    public static Proxy toJavaProxy() {
        if (!isEnabled()) return Proxy.NO_PROXY;
        return new Proxy(proxyType, new InetSocketAddress(proxyHost, proxyPort));
    }

    /**
     * 返回 "--proxy host:port" 格式（供 yt-dlp 命令行使用）
     */
    public static String toCliArgs() {
        if (!isEnabled()) return "";
        String scheme = proxyType == Proxy.Type.SOCKS ? "socks5://" : "http://";
        return "--proxy " + scheme + proxyHost + ":" + proxyPort;
    }

    public static String getProxyString() {
        if (!isEnabled()) return "无";
        String scheme = proxyType == Proxy.Type.SOCKS ? "socks5://" : "http://";
        return scheme + proxyHost + ":" + proxyPort;
    }

    // ==================== 代理验证 ====================

    /** 用于验证代理的目标 URL */
    private static final String TEST_URL = "https://x.com";

    /**
     * 测试代理是否可用 —— 通过代理访问 x.com 官网验证
     * @return 测试结果，包含是否成功、状态码、响应时间
     */
    public static ProxyTestResult testProxy() {
        if (!isEnabled()) {
            return new ProxyTestResult(false, -1, 0, "代理未启用");
        }

        long startTime = System.currentTimeMillis();
        try {
            URL url = new URL(TEST_URL);
            Proxy javaProxy = toJavaProxy();
            HttpURLConnection conn = (HttpURLConnection) url.openConnection(javaProxy);
            conn.setRequestMethod("HEAD");                     // HEAD 轻量请求
            conn.setRequestProperty("User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) XDownload/1.0");
            conn.setConnectTimeout(8000);
            conn.setReadTimeout(8000);
            conn.setInstanceFollowRedirects(true);

            int code = conn.getResponseCode();
            long elapsed = System.currentTimeMillis() - startTime;
            conn.disconnect();

            if (code >= 200 && code < 400) {
                return new ProxyTestResult(true, code, elapsed,
                        "代理可用，连接 x.com 成功");
            } else {
                return new ProxyTestResult(false, code, elapsed,
                        "x.com 返回异常状态码: " + code);
            }
        } catch (IOException e) {
            long elapsed = System.currentTimeMillis() - startTime;
            String msg = e.getMessage();
            if (msg != null && msg.contains("Connection refused")) {
                return new ProxyTestResult(false, -1, elapsed, "连接被拒绝，代理端口未开放");
            } else if (msg != null && msg.contains("timeout")) {
                return new ProxyTestResult(false, -1, elapsed, "连接超时，代理无响应");
            } else if (msg != null && msg.contains("UnknownHost")) {
                return new ProxyTestResult(false, -1, elapsed, "无法解析 x.com，请检查 DNS / 代理");
            }
            return new ProxyTestResult(false, -1, elapsed,
                    "代理连接失败: " + (msg != null ? msg : "未知错误"));
        }
    }

    /**
     * 代理测试结果
     */
    public static class ProxyTestResult {
        public final boolean success;
        public final int httpStatus;
        public final long elapsedMs;
        public final String message;

        public ProxyTestResult(boolean success, int httpStatus, long elapsedMs, String message) {
            this.success = success;
            this.httpStatus = httpStatus;
            this.elapsedMs = elapsedMs;
            this.message = message;
        }

        @Override
        public String toString() {
            if (success) {
                return "✓ " + message + " (" + elapsedMs + "ms)";
            } else {
                return "✗ " + message + " (耗时 " + elapsedMs + "ms)";
            }
        }
    }

    // ==================== 私有 ====================

    private static void loadFromSystemProperties() {
        // JVM 系统属性（-Dhttp.proxyHost=... -Dhttp.proxyPort=...）
        String host = System.getProperty("http.proxyHost");
        String port = System.getProperty("http.proxyPort");
        if (host != null && !host.isEmpty()) {
            proxyHost = host;
            try { proxyPort = Integer.parseInt(port); } catch (Exception e) { proxyPort = 8080; }
            enabled = true;
            return;
        }

        host = System.getProperty("https.proxyHost");
        port = System.getProperty("https.proxyPort");
        if (host != null && !host.isEmpty()) {
            proxyHost = host;
            try { proxyPort = Integer.parseInt(port); } catch (Exception e) { proxyPort = 8080; }
            enabled = true;
            return;
        }

        // 环境变量（HTTP_PROXY / HTTPS_PROXY）
        String envProxy = System.getenv("HTTP_PROXY");
        if (envProxy == null) envProxy = System.getenv("HTTPS_PROXY");
        if (envProxy == null) envProxy = System.getenv("http_proxy");
        if (envProxy == null) envProxy = System.getenv("https_proxy");

        if (envProxy != null && !envProxy.isEmpty()) {
            parseEnvProxy(envProxy);
        }
    }

    private static void parseEnvProxy(String proxy) {
        // 格式: http://host:port 或 host:port
        String stripped = proxy.replaceFirst("^https?://", "").replaceFirst("/$", "");
        String[] parts = stripped.split(":");
        if (parts.length >= 1) {
            proxyHost = parts[0];
            try { proxyPort = Integer.parseInt(parts[1]); } catch (Exception e) { proxyPort = 8080; }
            enabled = true;
        }
    }
}
