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

const BROWSERS = [
  { value: "none", label: "无" },
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
      toast.error("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp");
      return;
    }

    setValidating(true);
    setVerified(false);

    const unlisten = await listen<string>("cookies-progress", (event) => {
      toast.info(event.payload, { duration: 3000 });
    });

    try {
      const result = await validateCookies(selected);
      if (result.success) {
        toast.success(result.message);
        setVerified(true);
        setVerifiedUsername(result.username ?? null);
      } else {
        toast.error(result.message);
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
      toast.success(`Cookies 已保存并加载: ${selected}`);

      // Notify other pages (e.g. DownloadPage) to reload the latest config.
      window.dispatchEvent(new CustomEvent("config-applied"));
    } catch (err: any) {
      toast.error(`保存失败: ${err}`);
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
    statusText = `验证中: ${selected}`;
    statusColor = "text-amber-500";
  } else if (verified && selected !== loadedBrowser) {
    statusText = `已验证: ${selected}${
      verifiedUsername ? ` — @${verifiedUsername}` : ""
    }，请保存并加载生效`;
    statusColor = "text-green-600";
  } else if (loadedBrowser) {
    statusText = `已加载: ${loadedBrowser}${
      loadedUsername ? ` — @${loadedUsername}` : ""
    }`;
    statusColor = "text-green-600";
  } else {
    statusText = "无 cookies";
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
        <span className="text-xs text-gray-500">浏览器:</span>
        <select
          value={selected}
          onChange={(e) => handleSelect(e.target.value)}
          className="text-xs"
        >
          {BROWSERS.map((b) => (
            <option key={b.value} value={b.value}>
              {b.label}
            </option>
          ))}
        </select>
        <button className="btn" onClick={handleValidate} disabled={selected === "none" || validating}>
          {validating ? "验证中..." : "验证"}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={!verified || selected === loadedBrowser || saving}
        >
          <Save size={13} />
          {saving ? "保存中..." : "保存并加载"}
        </button>
      </div>
    </div>
  );
}
