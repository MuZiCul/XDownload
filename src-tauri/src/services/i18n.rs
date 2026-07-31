use crate::services::config::ConfigManager;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

// ==================== Language Constants ====================

/// 0 = zh (Chinese), 1 = en (English)
static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

/// String tables: initialized once.
static STRINGS: OnceLock<(HashMap<String, String>, HashMap<String, String>)> = OnceLock::new();

fn tables() -> &'static (HashMap<String, String>, HashMap<String, String>) {
    STRINGS.get_or_init(|| {
        let mut zh = HashMap::new();
        let mut en = HashMap::new();

        // ===== Tabs =====
        put(&mut zh, &mut en, "tab.download", "下载", "Download");
        put(&mut zh, &mut en, "tab.settings", "设置", "Settings");
        put(&mut zh, &mut en, "tab.log", "日志", "Log");
        put(&mut zh, &mut en, "tab.about", "关于", "About");

        // ===== Download Panel =====
        put(&mut zh, &mut en, "url.label", "链接:", "URL:");
        put(&mut zh, &mut en, "url.fetch", "获取信息", "Fetch Info");
        put(&mut zh, &mut en, "url.paste", "粘贴", "Paste");
        put(&mut zh, &mut en, "video.info", "视频信息", "Video Info");
        put(&mut zh, &mut en, "video.author", "作者:", "Author:");
        put(&mut zh, &mut en, "video.duration", "时长:", "Duration:");
        put(&mut zh, &mut en, "video.views", "播放:", "Views:");
        put(&mut zh, &mut en, "format.title", "格式列表", "Formats");
        put(&mut zh, &mut en, "format.best", "最佳", "Best");
        put(&mut zh, &mut en, "format.bestva", "视频+音频", "Video+Audio");
        put(&mut zh, &mut en, "format.audio", "仅音频", "Audio Only");
        put(&mut zh, &mut en, "opt.title", "配置状态", "Status");
        put(&mut zh, &mut en, "opt.dir", "输出目录:", "Output Dir:");
        put(&mut zh, &mut en, "settings.dir", "视频保存位置:", "Download Dir:");
        put(
            &mut zh,
            &mut en,
            "tools.hint",
            "提示: 下载慢或无法访问Github请先配置代理。也可手动下载解压到bin目录。",
            "Tip: slow or GitHub unreachable? Configure proxy first. Or extract to bin/ manually.",
        );
        put(&mut zh, &mut en, "opt.browse", "浏览", "Browse");
        put(&mut zh, &mut en, "opt.audio", "仅提取音频 (MP3)", "Extract Audio (MP3)");
        put(&mut zh, &mut en, "opt.retries", "重试次数:", "Retries:");
        put(&mut zh, &mut en, "opt.start", "状态信息", "Status");
        put(&mut zh, &mut en, "opt.download", "下载", "Download");
        put(&mut zh, &mut en, "opt.cancel", "取消", "CANCEL");

        // ===== Progress =====
        put(&mut zh, &mut en, "prog.fetching", "正在获取视频信息...", "Fetching video info...");
        put(&mut zh, &mut en, "prog.ready", "就绪", "Ready");
        put(&mut zh, &mut en, "prog.downloading", "正在下载...", "Downloading...");
        put(&mut zh, &mut en, "prog.cancelled", "已取消", "Cancelled");
        put(&mut zh, &mut en, "prog.complete", "下载完成", "Download complete");
        put(
            &mut zh,
            &mut en,
            "prog.complete.msg",
            "下载完成!\n保存至: ",
            "Download complete!\nSaved to: ",
        );
        put(&mut zh, &mut en, "prog.failed", "下载失败", "Download failed");
        put(&mut zh, &mut en, "prog.done.title", "完成", "Done");
        put(&mut zh, &mut en, "prog.error.title", "获取错误", "Fetch Error");
        put(&mut zh, &mut en, "prog.eta", "剩余:", "ETA: ");

        // ===== Settings - Proxy =====
        put(&mut zh, &mut en, "proxy.title", "代理", "Proxy");
        put(&mut zh, &mut en, "proxy.none", "无代理", "No Proxy");
        put(&mut zh, &mut en, "proxy.manual", "手动代理", "Manual Proxy");
        put(&mut zh, &mut en, "proxy.host", "主机:", "Host:");
        put(&mut zh, &mut en, "proxy.port", "端口:", "Port:");
        put(&mut zh, &mut en, "proxy.test", "测试代理", "Test Proxy");
        put(&mut zh, &mut en, "proxy.autodetect", "自动检测", "Auto Detect");
        put(&mut zh, &mut en, "proxy.testing", "测试中...", "Testing...");
        put(&mut zh, &mut en, "proxy.ok", "通过 - x.com ", "OK - x.com ");
        put(&mut zh, &mut en, "proxy.failed", "失败", "Failed");
        put(&mut zh, &mut en, "proxy.detecting", "检测中...", "Detecting...");
        put(
            &mut zh,
            &mut en,
            "proxy.overseas",
            "海外环境 - 无需代理",
            "Overseas - no proxy needed",
        );
        put(
            &mut zh,
            &mut en,
            "proxy.domestic",
            "国内环境 - 建议代理",
            "Domestic - proxy recommended",
        );
        put(&mut zh, &mut en, "proxy.disabled", "代理已禁用", "Proxy disabled");

        // ===== Settings - Cookies =====
        put(&mut zh, &mut en, "cookies.title", "Cookies", "Cookies");
        put(&mut zh, &mut en, "cookies.browser", "浏览器:", "Browser:");
        put(&mut zh, &mut en, "cookies.validate", "验证", "Validate");
        put(&mut zh, &mut en, "cookies.save", "保存并应用", "Save & Apply");
        put(&mut zh, &mut en, "cookies.validating", "验证中...", "Validating cookies...");
        put(&mut zh, &mut en, "cookies.scanning", "扫描中...", "Scanning...");
        put(&mut zh, &mut en, "cookies.saved", "已保存: ", "Saved: ");
        put(&mut zh, &mut en, "cookies.none", "无", "none");

        // ===== Settings - Language =====
        put(&mut zh, &mut en, "lang.title", "语言 / Language", "Language");
        put(
            &mut zh,
            &mut en,
            "lang.restart",
            "语言已更改。重启后完全生效。",
            "Language changed. Restart to apply fully.",
        );
        put(&mut zh, &mut en, "lang.zh", "中文", "Chinese");
        put(&mut zh, &mut en, "lang.en", "English", "English");

        // ===== About =====
        put(
            &mut zh,
            &mut en,
            "about.desc",
            "基于 yt-dlp 的视频下载器",
            "Video downloader based on yt-dlp",
        );
        put(&mut zh, &mut en, "about.ytdlp", "yt-dlp: ", "yt-dlp: ");
        put(&mut zh, &mut en, "about.ffmpeg", "ffmpeg: ", "ffmpeg: ");
        put(&mut zh, &mut en, "about.ffmpeg.ok", "可用", "Available");
        put(&mut zh, &mut en, "about.ffmpeg.no", "未找到", "Not found");
        put(&mut zh, &mut en, "about.update", "更新 yt-dlp", "Update yt-dlp");
        put(&mut zh, &mut en, "about.updating", "更新中...", "Updating...");
        put(&mut zh, &mut en, "about.update.title", "更新 yt-dlp", "Updating yt-dlp");

        // ===== Startup Wizard =====
        put(&mut zh, &mut en, "wizard.title", "首次运行设置", "First Run Setup");
        put(&mut zh, &mut en, "wizard.env", "环境检查", "Environment");
        put(&mut zh, &mut en, "wizard.proxy.title", "网络与代理", "Network & Proxy");
        put(&mut zh, &mut en, "wizard.cookies.title", "Cookies", "Cookies");
        put(&mut zh, &mut en, "wizard.next", "下一步 >", "Next >");
        put(&mut zh, &mut en, "wizard.back", "< 上一步", "< Back");
        put(&mut zh, &mut en, "wizard.skip", "跳过", "Skip");
        put(&mut zh, &mut en, "wizard.finish", "完成", "Finish");
        put(
            &mut zh,
            &mut en,
            "wizard.download.tools",
            "下载缺失工具",
            "Download Missing Tools",
        );
        put(&mut zh, &mut en, "wizard.ytdlp.checking", "yt-dlp: 检查中...", "yt-dlp: checking...");
        put(&mut zh, &mut en, "wizard.ffmpeg.checking", "ffmpeg: 检查中...", "ffmpeg: checking...");
        put(&mut zh, &mut en, "wizard.ytdlp.ok", "yt-dlp: 可用", "yt-dlp: OK");
        put(&mut zh, &mut en, "wizard.ytdlp.no", "yt-dlp: 未找到", "yt-dlp: NOT FOUND");
        put(&mut zh, &mut en, "wizard.ffmpeg.ok", "ffmpeg: 可用", "ffmpeg: OK");
        put(&mut zh, &mut en, "wizard.ffmpeg.no", "ffmpeg: 未找到", "ffmpeg: NOT FOUND");

        // ===== Settings Buttons =====
        put(&mut zh, &mut en, "settings.viewlog", "查看日志", "View Log");

        // ===== Status Bar =====
        put(&mut zh, &mut en, "status.proxy", "代理: ", "Proxy: ");
        put(&mut zh, &mut en, "status.cookies", "Cookies: ", "Cookies: ");
        put(&mut zh, &mut en, "status.none", "无", "none");
        put(&mut zh, &mut en, "status.ok", "可用", "OK");
        put(&mut zh, &mut en, "status.na", "无", "N/A");

        // ===== Common / Table =====
        put(&mut zh, &mut en, "common.ok", "确定", "OK");
        put(&mut zh, &mut en, "common.cancel", "取消", "Cancel");
        put(&mut zh, &mut en, "common.unknown", "未知", "Unknown");
        put(&mut zh, &mut en, "common.seconds", "秒", "s");
        put(&mut zh, &mut en, "table.id", "ID", "ID");
        put(&mut zh, &mut en, "table.ext", "扩展名", "Ext");
        put(&mut zh, &mut en, "table.res", "分辨率", "Resolution");
        put(&mut zh, &mut en, "table.size", "大小", "Size");
        put(&mut zh, &mut en, "table.type", "类型", "Type");

        // ===== Window title =====
        put(&mut zh, &mut en, "app.title", "X下载", "XDownload");

        (zh, en)
    })
}

fn put(zh: &mut HashMap<String, String>, en: &mut HashMap<String, String>, key: &str, zh_val: &str, en_val: &str) {
    zh.insert(key.to_string(), zh_val.to_string());
    en.insert(key.to_string(), en_val.to_string());
}

// ==================== Public API ====================

pub struct I18n;

impl I18n {
    /// Get the translated string for the given key.
    /// Falls back to the key itself if not found.
    pub fn get(key: &str) -> String {
        let (zh, en) = tables();
        let map = if CURRENT_LANG.load(Ordering::Relaxed) == 1 {
            en
        } else {
            zh
        };
        map.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    /// Get the current language code ("zh" or "en").
    pub fn get_lang() -> String {
        if CURRENT_LANG.load(Ordering::Relaxed) == 1 {
            "en".to_string()
        } else {
            "zh".to_string()
        }
    }

    /// Set the active language and persist it.
    pub fn set_lang(code: &str) {
        let val = match code {
            "en" => 1u8,
            _ => 0u8,
        };
        CURRENT_LANG.store(val, Ordering::Relaxed);
        let _ = ConfigManager::save_lang(code);
    }

    /// Load the saved language from config, falling back to auto-detection.
    pub fn load_saved() {
        if let Some(saved) = ConfigManager::load_lang() {
            match saved.as_str() {
                "en" => CURRENT_LANG.store(1, Ordering::Relaxed),
                _ => CURRENT_LANG.store(0, Ordering::Relaxed),
            }
        } else {
            // Auto-detect from timezone / system language
            Self::auto_detect();
        }
    }

    /// Auto-detect language from system timezone and locale.
    fn auto_detect() {
        // Check timezone for Chinese regions
        let tz = std::env::var("TZ").unwrap_or_default();
        let zh_tz = [
            "Asia/Shanghai",
            "Asia/Chongqing",
            "Asia/Harbin",
            "Asia/Urumqi",
            "Asia/Taipei",
            "Asia/Hong_Kong",
            "Asia/Macau",
            "Asia/Singapore",
        ];

        if zh_tz.iter().any(|t| tz.contains(t)) {
            CURRENT_LANG.store(0, Ordering::Relaxed);
            return;
        }

        // Check system user.language property
        if let Ok(sys_lang) = std::env::var("user.language") {
            if sys_lang.starts_with("zh") {
                CURRENT_LANG.store(0, Ordering::Relaxed);
                return;
            }
        }

        // On Windows, also check the system locale via the LANG environment variable
        // which some tools set to indicate the UI language
        if let Ok(lang_env) = std::env::var("LANG") {
            if lang_env.starts_with("zh") || lang_env.starts_with("zh_CN") {
                CURRENT_LANG.store(0, Ordering::Relaxed);
                return;
            }
        }

        // Default to English
        CURRENT_LANG.store(1, Ordering::Relaxed);
    }
}
