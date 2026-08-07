import { useState, useEffect } from "react";
import { testProxy, setProxyMode, getProxyStatus, loadSettings, saveSettings, applySavedProxy } from "../../lib/bindings";
import type { ProxyStatus, ProxyTestResult, AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save } from "lucide-react";
import { useI18n } from "../../lib/i18n";

// Cache the last proxy connectivity test keyed by (host, port, scheme) so that
// switching tabs (which remounts this component) does not re-test an unchanged
// proxy configuration on every visit to the settings page.
let cachedProxyTest: {
  host: string;
  port: number;
  scheme: string;
  state: "idle" | "success" | "error";
} = { host: "", port: 0, scheme: "http", state: "idle" };

type Props = {
  host?: string;
  port?: number;
  scheme?: string;
  onChange: (host: string, port: number) => void;
};

export default function ProxySetting({ host, port, scheme, onChange }: Props) {
  const { t } = useI18n();
  const [mode, setMode] = useState<"none" | "manual" | "system">("none");
  const [h, setH] = useState(host || "127.0.0.1");
  const [p, setP] = useState(port || 7890);
  const [sc, setSc] = useState(scheme || "http");
  const [sysProxyHost, setSysProxyHost] = useState<string | null>(null);
  const [sysProxyPort, setSysProxyPort] = useState(0);
  const [sysProxyStr, setSysProxyStr] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [testState, setTestState] = useState<"idle" | "testing" | "success" | "error">("idle");

  // Run a connectivity test and cache the outcome (keyed by proxy config).
  const runTest = (
    host: string,
    port: number,
    scheme: string
  ): Promise<ProxyTestResult> => {
    setTestState("testing");
    return testProxy(host, port, scheme).then(
      (result: ProxyTestResult) => {
        setTestState(result.success ? "success" : "error");
        cachedProxyTest = {
          host,
          port,
          scheme,
          state: result.success ? "success" : "error",
        };
        return result;
      },
      (err: any) => {
        setTestState("error");
        cachedProxyTest = { host, port, scheme, state: "error" };
        throw err;
      }
    );
  };

  // Track committed state to enable/disable save button
  const [committedMode, setCommittedMode] = useState<"none" | "manual" | "system">("none");
  const [committedHost, setCommittedHost] = useState(host || "127.0.0.1");
  const [committedPort, setCommittedPort] = useState(port || 7890);
  const [committedScheme, setCommittedScheme] = useState(scheme || "http");

  const effectiveHost = mode === "system" && sysProxyHost ? sysProxyHost : h;
  const effectivePort = mode === "system" && sysProxyPort ? sysProxyPort : p;
  const changed =
    mode !== committedMode ||
    effectiveHost !== committedHost ||
    effectivePort !== committedPort ||
    sc !== committedScheme;

  useEffect(() => {
    if (host) {
      setMode("manual");
      setH(host);
      setCommittedMode("manual");
      setCommittedHost(host);
    }
    if (port) {
      setP(port);
      setCommittedPort(port);
    }
    if (scheme) {
      setSc(scheme);
      setCommittedScheme(scheme);
    }
  }, [host, port, scheme]);

  // On mount: check for system proxy and saved preference
  useEffect(() => {
    getProxyStatus().then((status: ProxyStatus) => {
      if (status.enabled && status.from_system) {
        setSysProxyHost(status.host);
        setSysProxyPort(status.port);
        setSysProxyStr(status.proxy_string);
      }

      if (status.enabled && status.from_system) {
        setMode("system");
        setCommittedMode("system");
        setCommittedHost(status.host || "");
        setCommittedPort(status.port || 0);
        onChange(status.host || "", status.port || 0);
      } else if (status.enabled && !status.from_system) {
        setMode("manual");
        setCommittedMode("manual");
        if (status.host) {
          setH(status.host);
          setP(status.port);
          setCommittedHost(status.host);
          setCommittedPort(status.port);
        }
      }

      // 启动时主动测试代理连通性；代理配置未变时复用上次缓存结果，
      // 避免每次切到设置页都重复发起网络测试
      if (status.enabled && status.host && status.port > 0) {
        if (
          cachedProxyTest.host === status.host &&
          cachedProxyTest.port === status.port &&
          cachedProxyTest.scheme === sc
        ) {
          setTestState(cachedProxyTest.state);
        } else {
          runTest(status.host, status.port, sc);
        }
      }
    }).catch(() => {});
  }, []);

  const handleModeChange = (newMode: "none" | "manual" | "system") => {
    setMode(newMode);

    if (newMode === "none") {
      setProxyMode(false);
      onChange("", 0);
    } else if (newMode === "system") {
      setProxyMode(true);
      if (sysProxyHost) {
        onChange(sysProxyHost, sysProxyPort);
      } else {
        toast.warning(t("proxy.noSystem"));
      }
    } else {
      setProxyMode(true);
      if (h && p) {
        onChange(h, p);
      }
    }
  };

  const handleTest = async () => {
    if (!effectiveHost) {
      toast.warning(t("proxy.hostRequired"));
      return;
    }
    try {
      const result = await runTest(effectiveHost, effectivePort, sc);
      if (result.success) {
        onChange(effectiveHost, effectivePort);
        toast.success(t("proxy.testPassed", { ms: result.elapsed_ms }));
      } else {
        toast.error(t("proxy.testFail", { msg: result.message }));
      }
    } catch (err: any) {
      toast.error(`${err}`);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const cfg: AppSettings = await loadSettings();
      if (mode === "none") {
        cfg.proxy_host = undefined;
        cfg.proxy_port = undefined;
        cfg.proxy_scheme = undefined;
      } else {
        cfg.proxy_host = effectiveHost;
        cfg.proxy_port = effectivePort;
        cfg.proxy_scheme = sc;
      }
      await saveSettings(cfg);

      if (mode === "none") {
        setProxyMode(false);
        toast.success(t("proxy.disabledSaved"));
      } else {
        // Apply the just-saved proxy to runtime explicitly. Testing is a
        // separate concern (the "测试" button / mount-time check) and is NOT
        // part of saving, so saving stays fast and never misleads.
        await applySavedProxy();
        toast.success(t("proxy.savedApplied"));
      }

      // Mark as clean
      setCommittedMode(mode);
      setCommittedHost(effectiveHost);
      setCommittedPort(effectivePort);
      setCommittedScheme(sc);

      // Notify other pages (e.g. DownloadPage) to reload the latest config.
      window.dispatchEvent(new CustomEvent("config-applied"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  const manualDisabled = mode !== "manual";

  return (
    <div className="section-card">
      <div className="section-title">
        {t("proxy.title")}
        {testState === "testing" && (
          <span className="normal-case font-normal text-[10px] text-yellow-600 ml-2">{t("proxy.testing")}</span>
        )}
        {testState === "success" && (
          <span className="normal-case font-normal text-[10px] text-green-600 ml-2">{t("proxy.ok")}</span>
        )}
        {testState === "error" && (
          <span className="normal-case font-normal text-[10px] text-red-600 ml-2">{t("proxy.error")}</span>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input type="radio" name="proxyMode" checked={mode === "none"} onChange={() => handleModeChange("none")} className="size-3" />
          {t("proxy.none")}
        </label>
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input type="radio" name="proxyMode" checked={mode === "manual"} onChange={() => handleModeChange("manual")} className="size-3" />
          {t("proxy.manual")}
        </label>
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input type="radio" name="proxyMode" checked={mode === "system"} onChange={() => handleModeChange("system")} className="size-3" />
          {t("proxy.system")}
        </label>
        <span className="text-xs text-gray-500">{t("proxy.type")}</span>
        <select
          value={sc}
          onChange={(e) => {
            setSc(e.target.value);
            setTestState("idle");
          }}
          disabled={manualDisabled}
          className="text-xs px-1"
        >
          <option value="http">HTTP</option>
          <option value="socks5">SOCKS5</option>
        </select>
        <span className="text-xs text-gray-500">{t("proxy.host")}</span>
        <input
          type="text"
          value={effectiveHost}
          onChange={(e) => { setH(e.target.value); setTestState("idle"); }}
          disabled={manualDisabled}
          className="[field-sizing:content]"
        />
        <span className="text-xs text-gray-500">{t("proxy.port")}</span>
        <input
          type="text"
          inputMode="numeric"
          pattern="[0-9]*"
          value={effectivePort}
          onChange={(e) => {
            const v = e.target.value.replace(/\D/g, "");
            setP(v ? parseInt(v, 10) : 7890);
            setTestState("idle");
          }}
          disabled={manualDisabled}
          className="[field-sizing:content]"
        />
        <button className="btn text-xs px-2" onClick={handleTest} disabled={mode === "none"}>
          {t("proxy.test")}
        </button>
        <button
          className="btn flex items-center gap-1 text-xs px-2"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={12} />
          {saving ? "..." : t("common.save")}
        </button>
      </div>
    </div>
  );
}
