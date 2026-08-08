import { useState, useEffect, useCallback } from "react";
import {
  loadSettings,
} from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import DirSetting from "./DirSetting";
import ProxySetting from "./ProxySetting";
import CookiesSetting from "./CookiesSetting";
import ToolsSetting from "./ToolsSetting";
import MultiTaskSetting from "./MultiTaskSetting";
import LanguageSetting from "./LanguageSetting";
import ConfigButtons from "./ConfigButtons";
import { useI18n } from "../../lib/i18n";

export default function SettingsPage() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<AppSettings>({});
  const [loaded, setLoaded] = useState(false);
  // Bump on every "应用配置" to force children with internal state
  // (ProxySetting, CookiesSetting) to unmount+remount from fresh props.
  const [applyKey, setApplyKey] = useState(0);

  // Called by ConfigButtons after applying saved config.
  const handleApply = useCallback((fresh: AppSettings) => {
    console.log("[SettingsPage] handleApply, fresh:", JSON.stringify(fresh));
    setSettings(fresh);
    setApplyKey((k) => k + 1);
  }, []);

  useEffect(() => {
    loadSettings()
      .then((s) => {
        setSettings(s);
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
          <ConfigButtons settings={settings} onApply={handleApply} />

          <DirSetting
            key={`dir-${applyKey}`}
            dir={settings.download_dir ?? "downloads"}
            onChange={(d) => setSettings((s) => ({ ...s, download_dir: d }))}
          />
          <ProxySetting
            key={`proxy-${applyKey}`}
            host={settings.proxy_host}
            port={settings.proxy_port}
            scheme={settings.proxy_scheme}
            onChange={(host, port) =>
              setSettings((s) => ({ ...s, proxy_host: host, proxy_port: port }))
            }
          />
          <CookiesSetting
            key={`cookies-${applyKey}`}
            browser={settings.cookies_from_browser}
            onChange={(b) => setSettings((s) => ({ ...s, cookies_from_browser: b }))}
          />
          <ToolsSetting />

          <MultiTaskSetting
            key={`mt-${applyKey}`}
            concurrency={settings.concurrency ?? 1}
            retryCount={settings.retry_count ?? 0}
            queuePersist={settings.queue_persist ?? false}
            onChange={(patch) =>
              setSettings((s) => ({ ...s, ...patch }))
            }
          />

          <LanguageSetting
            key={`lang-${applyKey}`}
            lang={settings.lang ?? "zh"}
            onChange={(l) => setSettings((s) => ({ ...s, lang: l }))}
          />

        </div>
      )}
    </div>
  );
}
