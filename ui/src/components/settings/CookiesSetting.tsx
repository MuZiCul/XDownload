import { useState, useEffect } from "react";
import {
  validateCookies,
  scanCookies,
  saveAndApplyCookies,
  loadSavedCookies,
  checkYtdlp,
} from "../../lib/bindings";
import { toast } from "sonner";
import { Save } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../../lib/i18n";

const BROWSERS = [
  { value: "none", label: "cookies.none" },
  { value: "chrome", label: "Chrome" },
  { value: "firefox", label: "Firefox" },
  { value: "edge", label: "Edge" },
  { value: "brave", label: "Brave" },
  { value: "opera", label: "Opera" },
];

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

  useEffect(() => {
    if (browser) setSelected(browser);
  }, [browser]);

  // On mount: load saved config
  useEffect(() => {
    loadSavedCookies()
      .then(([savedBrowser]) => {
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
      await saveAndApplyCookies(selected);
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
      <div className="section-title">
        Cookies
        <span className={`normal-case font-normal text-[10px] ${statusColor} ml-2`}>
          ● {statusText}
        </span>
      </div>
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-xs text-gray-500">{t("cookies.browser")}</span>
        <select
          value={selected}
          onChange={(e) => handleSelect(e.target.value)}
          className="text-xs"
        >
          {BROWSERS.map((b) => (
            <option key={b.value} value={b.value}>
              {t(b.label)}
            </option>
          ))}
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
