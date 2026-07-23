package util;

import java.util.LinkedHashMap;
import java.util.Map;

/**
 * 国际化：中英双语，可运行时切换
 */
public class I18n {

    private static String lang = "zh"; // 默认中文

    private static final Map<String, String> ZH = new LinkedHashMap<>();
    private static final Map<String, String> EN = new LinkedHashMap<>();

    static {
        // ===== 标签页 =====
        put("tab.download", "下载", "Download");
        put("tab.settings", "设置", "Settings");
        put("tab.about", "关于", "About");

        // ===== 下载面板 =====
        put("url.label", "链接:", "URL:");
        put("url.fetch", "获取信息", "Fetch Info");
        put("url.paste", "粘贴", "Paste");
        put("video.info", "视频信息", "Video Info");
        put("video.author", "作者:", "Author:");
        put("video.duration", "时长:", "Duration:");
        put("video.views", "播放:", "Views:");
        put("format.title", "格式列表", "Formats");
        put("format.best", "最佳", "Best");
        put("format.bestva", "视频+音频", "Video+Audio");
        put("format.audio", "仅音频", "Audio Only");
        put("opt.title", "配置状态", "Status");
        put("opt.dir", "输出目录:", "Output Dir:");
        put("settings.dir", "视频保存位置:", "Download Dir:");
        put("tools.hint", "提示: 下载慢或无法访问Github请先配置代理。也可手动下载解压到bin目录。",
                "Tip: slow or GitHub unreachable? Configure proxy first. Or extract to bin/ manually.");
        put("opt.browse", "浏览", "Browse");
        put("opt.audio", "仅提取音频 (MP3)", "Extract Audio (MP3)");
        put("opt.retries", "重试次数:", "Retries:");
        put("opt.start", "状态信息", "Status");
        put("opt.download", "下载", "Download");
        put("opt.cancel", "取消", "CANCEL");

        // ===== 进度 =====
        put("prog.fetching", "正在获取视频信息...", "Fetching video info...");
        put("prog.ready", "就绪", "Ready");
        put("prog.downloading", "正在下载...", "Downloading...");
        put("prog.cancelled", "已取消", "Cancelled");
        put("prog.complete", "下载完成", "Download complete");
        put("prog.complete.msg", "下载完成!\n保存至: ", "Download complete!\nSaved to: ");
        put("prog.failed", "下载失败", "Download failed");
        put("prog.done.title", "完成", "Done");
        put("prog.error.title", "获取错误", "Fetch Error");

        // ===== 设置 - 代理 =====
        put("proxy.title", "代理", "Proxy");
        put("proxy.none", "无代理", "No Proxy");
        put("proxy.manual", "手动代理", "Manual Proxy");
        put("proxy.host", "主机:", "Host:");
        put("proxy.port", "端口:", "Port:");
        put("proxy.test", "测试代理", "Test Proxy");
        put("proxy.autodetect", "自动检测", "Auto Detect");
        put("proxy.testing", "测试中...", "Testing...");
        put("proxy.ok", "通过 - x.com ", "OK - x.com ");
        put("proxy.failed", "失败", "Failed");
        put("proxy.detecting", "检测中...", "Detecting...");
        put("proxy.overseas", "海外环境 - 无需代理", "Overseas - no proxy needed");
        put("proxy.domestic", "国内环境 - 建议代理", "Domestic - proxy recommended");
        put("proxy.disabled", "代理已禁用", "Proxy disabled");

        // ===== 设置 - Cookies =====
        put("cookies.title", "Cookies", "Cookies");
        put("cookies.browser", "浏览器:", "Browser:");
        put("cookies.validate", "验证", "Validate");
        put("cookies.save", "保存并应用", "Save & Apply");
        put("cookies.validating", "验证中...", "Validating cookies...");
        put("cookies.scanning", "扫描中...", "Scanning...");
        put("cookies.saved", "已保存: ", "Saved: ");
        put("cookies.none", "无", "none");

        // ===== 设置 - 语言 =====
        put("lang.title", "语言 / Language", "Language");
        put("lang.restart", "语言已更改。重启后完全生效。", "Language changed. Restart to apply fully.");
        put("lang.zh", "中文", "Chinese");
        put("lang.en", "English", "English");

        // ===== 关于 =====
        put("about.desc", "基于 yt-dlp 的视频下载器", "Video downloader based on yt-dlp");
        put("about.ytdlp", "yt-dlp: ", "yt-dlp: ");
        put("about.ffmpeg", "ffmpeg: ", "ffmpeg: ");
        put("about.ffmpeg.ok", "可用", "Available");
        put("about.ffmpeg.no", "未找到", "Not found");
        put("about.update", "更新 yt-dlp", "Update yt-dlp");
        put("about.updating", "更新中...", "Updating...");
        put("about.update.title", "更新 yt-dlp", "Updating yt-dlp");

        // ===== 启动引导 =====
        put("wizard.title", "首次运行设置", "First Run Setup");
        put("wizard.env", "环境检查", "Environment");
        put("wizard.proxy.title", "网络与代理", "Network & Proxy");
        put("wizard.cookies.title", "Cookies", "Cookies");
        put("wizard.next", "下一步 >", "Next >");
        put("wizard.back", "< 上一步", "< Back");
        put("wizard.skip", "跳过", "Skip");
        put("wizard.finish", "完成", "Finish");
        put("wizard.download.tools", "下载缺失工具", "Download Missing Tools");
        put("wizard.ytdlp.checking", "yt-dlp: 检查中...", "yt-dlp: checking...");
        put("wizard.ffmpeg.checking", "ffmpeg: 检查中...", "ffmpeg: checking...");
        put("wizard.ytdlp.ok", "yt-dlp: 可用", "yt-dlp: OK");
        put("wizard.ytdlp.no", "yt-dlp: 未找到", "yt-dlp: NOT FOUND");
        put("wizard.ffmpeg.ok", "ffmpeg: 可用", "ffmpeg: OK");
        put("wizard.ffmpeg.no", "ffmpeg: 未找到", "ffmpeg: NOT FOUND");

        // ===== 状态栏 =====
        put("status.proxy", "代理: ", "Proxy: ");
        put("status.cookies", "Cookies: ", "Cookies: ");
        put("status.none", "无", "none");
        put("status.ok", "可用", "OK");
        put("status.na", "无", "N/A");

        // ===== 通用 =====
        put("common.ok", "确定", "OK");
        put("common.cancel", "取消", "Cancel");
        put("common.unknown", "未知", "Unknown");
        put("common.seconds", "秒", "s");
        put("prog.eta", "剩余:", "ETA: ");
        put("table.id", "ID", "ID");
        put("table.ext", "扩展名", "Ext");
        put("table.res", "分辨率", "Resolution");
        put("table.size", "大小", "Size");
        put("table.type", "类型", "Type");
    }

    private static void put(String key, String zh, String en) { ZH.put(key, zh); EN.put(key, en); }

    public static String get(String key) {
        Map<String, String> map = "en".equals(lang) ? EN : ZH;
        String v = map.get(key);
        return v != null ? v : key;
    }

    public static String getLang() { return lang; }

    public static void setLang(String l) {
        if ("en".equals(l) || "zh".equals(l)) {
            lang = l;
            ConfigManager.saveLang(l);
        }
    }

    public static void loadSaved() {
        String saved = ConfigManager.loadLang();
        if (saved != null) {
            lang = saved;
        } else {
            // 未保存时根据系统时区自动检测：中国时区 → 中文，否则 → 英文
            String tz = java.util.TimeZone.getDefault().getID();
            if (tz != null && (tz.contains("Asia/Shanghai") || tz.contains("Asia/Chongqing")
                    || tz.contains("Asia/Harbin") || tz.contains("Asia/Urumqi")
                    || tz.contains("Asia/Taipei") || tz.contains("Asia/Hong_Kong")
                    || tz.contains("Asia/Macau") || tz.contains("Asia/Singapore"))) {
                lang = "zh";
            } else {
                // 也检查系统语言
                String sysLang = System.getProperty("user.language");
                if (sysLang != null && sysLang.startsWith("zh")) {
                    lang = "zh";
                } else {
                    lang = "en";
                }
            }
        }
    }
}
