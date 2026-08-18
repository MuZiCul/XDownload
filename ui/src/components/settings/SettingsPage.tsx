import { useState, useEffect } from "react";
import {
  loadSettings,
  getDownloadDir,
} from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import DirSetting from "./DirSetting";
import ProxySetting from "./ProxySetting";
import CookiesSetting from "./CookiesSetting";
import ToolsSetting from "./ToolsSetting";
import MultiTaskSetting from "./MultiTaskSetting";
import BookmarksSetting from "./BookmarksSetting";
import HlsSetting from "./HlsSetting";
import LanguageSetting from "./LanguageSetting";
import ConfigButtons from "./ConfigButtons";
import StatisticsPage from "../stats/StatisticsPage";
import LogViewerPage from "../logs/LogViewerPage";
import { useI18n } from "../../lib/i18n";

export default function SettingsPage() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<AppSettings>({});
  const [loaded, setLoaded] = useState(false);
  // 统计页覆盖层开关（设置页「统计」按钮）。
  const [statsOpen, setStatsOpen] = useState(false);
  // 应用内日志页覆盖层开关（设置页「软件日志」按钮）。
  const [logsOpen, setLogsOpen] = useState(false);
  // 展示用的下载目录（绝对路径）：来自后端 get_download_dir（空/相对配置
  // 会归一化为 <root>/downloads 的绝对路径）。仅显示层，配置值本身不改写。
  const [displayDir, setDisplayDir] = useState("");

  useEffect(() => {
    Promise.all([loadSettings(), getDownloadDir()])
      .then(([s, dir]) => {
        setSettings(s);
        setDisplayDir(dir);
        setLoaded(true);
      })
      .catch(() => setLoaded(true));
  }, []);

  return (
    <div className="p-3 max-w-[900px] mx-auto">
      {!loaded ? (
        <div className="flex items-center justify-center h-40 text-zinc-400 text-sm">
          {t("common.loading")}
        </div>
      ) : (
        <div className="space-y-3">
          <ConfigButtons
            onOpenLogs={() => setLogsOpen(true)}
            onOpenStats={() => setStatsOpen(true)}
          />

          <DirSetting
            dir={displayDir || settings.download_dir || "downloads"}
            onChange={(d) => {
              setSettings((s) => ({ ...s, download_dir: d }));
              setDisplayDir(d);
            }}
          />
          <ProxySetting
            host={settings.proxy_host}
            port={settings.proxy_port}
            scheme={settings.proxy_scheme}
            onChange={(host, port) =>
              setSettings((s) => ({ ...s, proxy_host: host, proxy_port: port }))
            }
          />
          <CookiesSetting
            browser={settings.cookies_from_browser}
            onChange={(b) => setSettings((s) => ({ ...s, cookies_from_browser: b }))}
          />
          <ToolsSetting
            useProxy={settings.tools_use_proxy ?? false}
            onChange={(v) => setSettings((s) => ({ ...s, tools_use_proxy: v }))}
          />

          <MultiTaskSetting
            concurrency={settings.concurrency ?? 1}
            retryCount={settings.retry_count ?? 0}
            queuePersist={settings.queue_persist ?? false}
            keepAwake={settings.keep_awake ?? false}
            rateLimit={settings.download_rate_limit ?? ""}
            onChange={(patch) =>
              setSettings((s) => ({ ...s, ...patch }))
            }
          />

          <BookmarksSetting />

          <HlsSetting
            concurrent={settings.hls_concurrent_fragments ?? 4}
            retries={settings.hls_fragment_retries ?? 10}
            onChange={(patch) =>
              setSettings((s) => ({ ...s, ...patch }))
            }
          />

          <LanguageSetting
            lang={settings.lang ?? "zh"}
            onChange={(l) => setSettings((s) => ({ ...s, lang: l }))}
          />

        </div>
      )}

      {/* 统计页覆盖层（设置页「统计」按钮打开） */}
      {statsOpen && <StatisticsPage onClose={() => setStatsOpen(false)} />}

      {/* 应用内日志页覆盖层（设置页「软件日志」按钮打开） */}
      {logsOpen && <LogViewerPage onClose={() => setLogsOpen(false)} />}
    </div>
  );
}
