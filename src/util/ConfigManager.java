package util;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * 持久化配置管理 —— 保存/加载代理、Cookies 等用户设置到 config/settings.json
 */
public class ConfigManager {

    private static final Path CONFIG_DIR = Paths.get(System.getProperty("user.dir"))
            .toAbsolutePath().resolve("config");
    private static final Path CONFIG_FILE = CONFIG_DIR.resolve("settings.json");

    // ==================== 加载 ====================

    /**
     * 加载配置，返回键值 Map；文件不存在则返回空 Map
     */
    public static Map<String, String> load() {
        Map<String, String> map = new LinkedHashMap<>();
        if (!Files.exists(CONFIG_FILE)) return map;

        try {
            String json = Files.readString(CONFIG_FILE);
            // 简易 JSON 解析（不引入第三方库）
            // 格式: { "key": "value", ... }
            int start = json.indexOf('{');
            int end = json.lastIndexOf('}');
            if (start < 0 || end < 0) return map;

            String body = json.substring(start + 1, end);
            // 匹配 "key": "value" 或 "key": 123
            Pattern p = Pattern.compile("\"([^\"]+)\"\\s*:\\s*\"([^\"]*)\"");
            Matcher m = p.matcher(body);
            while (m.find()) {
                map.put(m.group(1), m.group(2));
            }

            // 也匹配数字值: "key": 123
            Pattern numP = Pattern.compile("\"([^\"]+)\"\\s*:\\s*(\\d+)");
            Matcher numM = numP.matcher(body);
            while (numM.find()) {
                map.putIfAbsent(numM.group(1), numM.group(2));
            }
        } catch (IOException e) {
            System.err.println("  ⚠ 加载配置失败: " + e.getMessage());
        }
        return map;
    }

    // ==================== 保存 ====================

    /**
     * 保存配置到文件
     */
    public static void save(Map<String, String> config) {
        try {
            Files.createDirectories(CONFIG_DIR);

            StringBuilder sb = new StringBuilder();
            sb.append("{\n");
            int i = 0;
            for (Map.Entry<String, String> e : config.entrySet()) {
                sb.append("  \"");
                sb.append(escapeJson(e.getKey()));
                sb.append("\": \"");
                sb.append(escapeJson(e.getValue()));
                sb.append("\"");
                if (++i < config.size()) sb.append(",");
                sb.append("\n");
            }
            sb.append("}\n");

            Files.writeString(CONFIG_FILE, sb.toString());
        } catch (IOException e) {
            System.err.println("  ⚠ 保存配置失败: " + e.getMessage());
        }
    }

    private static String escapeJson(String s) {
        return s.replace("\\", "\\\\").replace("\"", "\\\"");
    }

    // ==================== 便捷方法 ====================

    /**
     * 从配置加载并应用代理设置
     * @return 是否成功加载并应用了代理
     */
    public static boolean applySavedProxy() {
        Map<String, String> cfg = load();
        String host = cfg.get("proxyHost");
        String portStr = cfg.get("proxyPort");
        if (host != null && !host.isEmpty() && portStr != null && !portStr.isEmpty()) {
            try {
                int port = Integer.parseInt(portStr);
                ProxyConfig.setProxy(host, port);
                return true;
            } catch (NumberFormatException ignored) {}
        }
        return false;
    }

    /**
     * 从配置加载 cookies 默认值
     * @return [browser, cookiesFile]，browser 为 null 表示未配置
     */
    public static String[] loadSavedCookies() {
        Map<String, String> cfg = load();
        String browser = cfg.get("cookiesFromBrowser");
        String file = cfg.get("cookiesFile");
        if (browser != null && !browser.isEmpty()) return new String[]{browser, null};
        if (file != null && !file.isEmpty()) return new String[]{null, file};
        return new String[]{null, null};
    }

    /**
     * 保存代理到配置
     */
    public static void saveProxy(String host, int port) {
        Map<String, String> cfg = load();
        cfg.put("proxyHost", host);
        cfg.put("proxyPort", String.valueOf(port));
        save(cfg);
    }

    /**
     * 保存 cookies 到配置
     */
    public static void saveCookies(String browser, String cookiesFile) {
        Map<String, String> cfg = load();
        if (browser != null && !browser.isEmpty()) {
            cfg.put("cookiesFromBrowser", browser);
            cfg.remove("cookiesFile");
        } else if (cookiesFile != null && !cookiesFile.isEmpty()) {
            cfg.put("cookiesFile", cookiesFile);
            cfg.remove("cookiesFromBrowser");
        }
        save(cfg);
    }

    /**
     * 从配置中移除代理
     */
    public static void removeProxy() {
        Map<String, String> cfg = load();
        cfg.remove("proxyHost");
        cfg.remove("proxyPort");
        save(cfg);
    }

    /**
     * 清除 cookies 配置
     */
    public static void clearCookies() {
        Map<String, String> cfg = load();
        cfg.remove("cookiesFromBrowser");
        cfg.remove("cookiesFile");
        save(cfg);
    }

    /**
     * 配置文件的路径（供外部显示用）
     */
    public static Path getConfigFile() {
        return CONFIG_FILE;
    }
}
