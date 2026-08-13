import { useState, useEffect } from "react";
import {
  validateCookies,
  scanCookies,
  listBrowsers,
  saveCookieSource,
  loadCookieSource,
  checkYtdlp,
} from "../../lib/bindings";
import { toast } from "sonner";
import { Save } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

/** 全部受支持浏览器（用于安装检测失败时的降级回退）。 */
const ALL_BROWSERS = [
  { value: "chrome", label: "Chrome" },
  { value: "firefox", label: "Firefox" },
  { value: "edge", label: "Edge" },
  { value: "brave", label: "Brave" },
  { value: "opera", label: "Opera" },
];

/** 浏览器显示名（i18n key 优先，否则用默认名）。 */
const BROWSER_LABELS: Record<string, string> = {
  chrome: "Chrome",
  firefox: "Firefox",
  edge: "Edge",
  brave: "Brave",
  opera: "Opera",
};

type Props = {
  browser?: string;
  onChange: (browser: string | undefined) => void;
};

export default function CookiesSetting({ browser, onChange }: Props) {
  const { t } = useI18n();
  const [selected, setSelected] = useState(browser || "none");
  const [validating, setValidating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loadedBrowser, setLoadedBrowser] = useState<string | null>(null);
  const [verified, setVerified] = useState(false);
  const [verifiedUsername, setVerifiedUsername] = useState<string | null>(null);
  const [loadedUsername, setLoadedUsername] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);
  /** 已安装浏览器列表；未加载到（检测失败）时降级为全部浏览器。 */
  const [installed, setInstalled] = useState<string[] | null>(null);

  useEffect(() => {
    if (browser) setSelected(browser);
  }, [browser]);

  // 下拉框选项：检测已安装的浏览器，只展示它们；检测失败时回退全量。
  const options = (installed ?? ALL_BROWSERS.map((b) => b.value)).map((value) => ({
    value,
    label: BROWSER_LABELS[value] ?? value,
  }));

  // On mount: load installed browsers + saved config
  useEffect(() => {
    listBrowsers()
      .then((list) => {
        // 只保留已知的浏览器名；后端返回顺序即展示顺序。
        const known = list.filter((b) => BROWSER_LABELS[b]);
        setInstalled(known.length > 0 ? known : null);
      })
      .catch(() => setInstalled(null));

    loadCookieSource()
      .then((savedBrowser) => {
        if (savedBrowser) {
          setLoadedBrowser(savedBrowser);
          setSelected(savedBrowser);
          setVerified(true); // already saved = already verified
        } else {
          scanCookies()
            .then((found) => {
              if (found) {
                setSelected(found);
                onChange(found);
              }
            })
            .catch(() => {});
        }
      })
      .catch(() => {})
      .finally(() => setInitialized(true));
  }, []);

  const handleSelect = (value: string) => {
    setSelected(value);
    setVerified(false); // changing browser resets verification
    setVerifiedUsername(null); // changing browser resets verified username
    if (value === "none") {
      onChange(undefined);
    } else {
      onChange(value);
    }
  };

  const handleValidate = async () => {
    if (selected === "none") return;

    const ytStatus = await checkYtdlp();
    if (!ytStatus.available) {
      toast.error(t("tools.missing.ytdlp"));
      return;
    }

    setValidating(true);
    setVerified(false);

    const unlisten = await listen<any>("cookies-progress", (event) => {
      const p = event.payload ?? {};
      const browser = p.browser ?? selected;
      if (p.step === 1) {
        toast.loading(t("cookies.step1", { browser }), { id: "cookies-progress" });
      } else if (p.step === 2) {
        toast.loading(t("cookies.step2"), { id: "cookies-progress" });
      } else if (p.step === 3) {
        toast.loading(t("cookies.step3"), { id: "cookies-progress" });
      }
      // step 0 (failure) is surfaced by the validateCookies result below.
    });

    try {
      const result = await validateCookies(selected);
      if (result.success) {
        toast.success(
          t("cookies.verifiedOk", { user: `@${result.username ?? ""}` }),
          { id: "cookies-progress" }
        );
        setVerified(true);
        setVerifiedUsername(result.username ?? null);
      } else {
        const code = result.error_code ?? "unknown";
        toast.error(
          t(`cookies.error.${code}`, {
            browser: selected,
            msg: result.message ?? "",
          }),
          { id: "cookies-progress" }
        );
        setVerifiedUsername(null);
      }
    } catch (err: any) {
      toast.error(`${err}`);
      setVerifiedUsername(null);
    } finally {
      (await unlisten)();
      setValidating(false);
    }
  };

  const handleSave = async () => {
    if (selected === "none" || !verified || selected === loadedBrowser) return;
    setSaving(true);
    try {
      await saveCookieSource(selected);
      setLoadedBrowser(selected);
      setLoadedUsername(verifiedUsername); // carry verified username to loaded state
      onChange(selected);
      toast.success(t("cookies.saved", { browser: selected }));

      // Notify other pages (e.g. DownloadPage) to reload the latest config.
      window.dispatchEvent(new CustomEvent("config-applied"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  if (!initialized) return null;

  // Status indicator next to the "Cookies" title.
  // Priority: validating > verified-but-not-saved > loaded > none
  let statusText: string;
  let statusColor: string;
  if (validating) {
    statusText = t("cookies.statusValidating", { browser: selected });
    statusColor = "text-amber-500";
  } else if (verified && selected !== loadedBrowser) {
    statusText = t("cookies.statusVerified", {
      browser: selected,
      user: verifiedUsername ? ` — @${verifiedUsername}` : "",
    });
    statusColor = "text-green-600";
  } else if (loadedBrowser) {
    statusText = t("cookies.statusLoaded", {
      browser: loadedBrowser,
      user: loadedUsername ? ` — @${loadedUsername}` : "",
    });
    statusColor = "text-green-600";
  } else {
    statusText = t("cookies.statusNone");
    statusColor = "text-gray-400";
  }

  return (
    <div className="section-card">
      <SectionTitle
        title={
          <>
            Cookies
            <span className={`normal-case font-normal text-[10px] ${statusColor} ml-2`}>
              ● {statusText}
            </span>
          </>
        }
        tip={t("cookies.hint")}
      />
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-xs text-gray-500">{t("cookies.browser")}</span>
        <select
          value={selected}
          onChange={(e) => handleSelect(e.target.value)}
          className="text-xs"
        >
          <option value="none">{t("cookies.none")}</option>
          {options.map((b) => (
            <option key={b.value} value={b.value}>
              {b.label}
            </option>
          ))}
          {/* 已保存的浏览器不在已安装列表中时保留显示，避免空白选择 */}
          {selected !== "none" && !options.some((o) => o.value === selected) && (
            <option value={selected}>{BROWSER_LABELS[selected] ?? selected}</option>
          )}
        </select>
        <button className="btn" onClick={handleValidate} disabled={selected === "none" || validating}>
          {validating ? t("cookies.validatingBtn") : t("cookies.validate")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={!verified || selected === loadedBrowser || saving}
        >
          <Save size={13} />
          {saving ? t("common.saving") : t("cookies.saveAndApply")}
        </button>
      </div>
    </div>
  );
}
