package util;

import java.net.HttpURLConnection;
import java.net.InetSocketAddress;
import java.net.Proxy;
import java.net.URL;

/**
 * 检测网络环境：国内外判断
 */
public class NetworkDetect {

    /** 用于判断是否在海外的测试站点（国内直连不通） */
    private static final String OVERSEAS_TEST = "https://www.google.com";

    /** 快速超时（毫秒） */
    private static final int QUICK_TIMEOUT = 3000;

    /**
     * 判断当前是否在海外（无需代理即可访问 Google）
     * @return true = 海外（不需要代理），false = 国内（需要代理）
     */
    public static boolean isOverseas() {
        try {
            URL url = new URL(OVERSEAS_TEST);
            HttpURLConnection conn = (HttpURLConnection) url.openConnection(Proxy.NO_PROXY);
            conn.setRequestMethod("HEAD");
            conn.setConnectTimeout(QUICK_TIMEOUT);
            conn.setReadTimeout(QUICK_TIMEOUT);
            conn.setInstanceFollowRedirects(false);
            conn.connect();
            int code = conn.getResponseCode();
            conn.disconnect();
            // 只要能连上（任何状态码），说明在海外
            return code > 0;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * 检测是否能直连 GitHub（用于判断下载 yt-dlp 是否需要代理）
     * @return true 表示可以直连
     */
    public static boolean isGithubAccessible() {
        try {
            URL url = new URL("https://github.com");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection(Proxy.NO_PROXY);
            conn.setRequestMethod("HEAD");
            conn.setConnectTimeout(5000);
            conn.setReadTimeout(5000);
            conn.setInstanceFollowRedirects(false);
            conn.connect();
            int code = conn.getResponseCode();
            conn.disconnect();
            return code > 0;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * 检测是否能访问 x.com（含代理），用于 fetch 前快速预检避免 yt-dlp 超时等待。
     * @return true 表示 x.com 可达
     */
    public static boolean isXAccessible() {
        try {
            URL url = new URL("https://x.com");
            Proxy javaProxy = ProxyConfig.isEnabled() ? ProxyConfig.toJavaProxy() : Proxy.NO_PROXY;
            HttpURLConnection conn = (HttpURLConnection) url.openConnection(javaProxy);
            conn.setRequestMethod("HEAD");
            conn.setConnectTimeout(5000);
            conn.setReadTimeout(5000);
            conn.setInstanceFollowRedirects(false);
            conn.connect();
            int code = conn.getResponseCode();
            conn.disconnect();
            return code > 0;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * 测试指定代理是否可用
     * @return 延迟毫秒，-1 表示不可用
     */
    public static long testProxyLatency(String host, int port) {
        try {
            long start = System.currentTimeMillis();
            Proxy proxy = new Proxy(Proxy.Type.HTTP, new InetSocketAddress(host, port));
            URL url = new URL(OVERSEAS_TEST);
            HttpURLConnection conn = (HttpURLConnection) url.openConnection(proxy);
            conn.setRequestMethod("HEAD");
            conn.setConnectTimeout(5000);
            conn.setReadTimeout(5000);
            conn.connect();
            int code = conn.getResponseCode();
            conn.disconnect();
            if (code > 0) {
                return System.currentTimeMillis() - start;
            }
        } catch (Exception ignored) {}
        return -1;
    }
}
