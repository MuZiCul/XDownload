import { useState, useEffect } from "react";
import { testProxy, setProxyMode, getProxyStatus, loadSettings, saveSettings } from "../../lib/bindings";
import type { ProxyStatus, AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save } from "lucide-react";

type Props = {
  host?: string;
  port?: number;
  onChange: (host: string, port: number) => void;
};

export default function ProxySetting({ host, port, onChange }: Props) {
  const [mode, setMode] = useState<"none" | "manual" | "system">("none");
  const [h, setH] = useState(host || "127.0.0.1");
  const [p, setP] = useState(port || 7890);
  const [sysProxyHost, setSysProxyHost] = useState<string | null>(null);
  const [sysProxyPort, setSysProxyPort] = useState(0);
  const [sysProxyStr, setSysProxyStr] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Track committed state to enable/disable save button
  const [committedMode, setCommittedMode] = useState<"none" | "manual" | "system">("none");
  const [committedHost, setCommittedHost] = useState(host || "127.0.0.1");
  const [committedPort, setCommittedPort] = useState(port || 7890);

  const effectiveHost = mode === "system" && sysProxyHost ? sysProxyHost : h;
  const effectivePort = mode === "system" && sysProxyPort ? sysProxyPort : p;
  const changed = mode !== committedMode || effectiveHost !== committedHost || effectivePort !== committedPort;

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
  }, [host, port]);

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
        toast.warning("未检测到系统代理，请先开启系统代理或选择手动代理");
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
      toast.warning("请输入代理主机地址");
      return;
    }
    try {
      const result = await testProxy(effectiveHost, effectivePort);
      if (result.success) {
        onChange(effectiveHost, effectivePort);
        toast.success(`代理测试通过 (${result.elapsed_ms}ms)`);
      } else {
        toast.error(result.message);
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
      } else {
        cfg.proxy_host = effectiveHost;
        cfg.proxy_port = effectivePort;
      }
      await saveSettings(cfg);

      if (mode === "none") {
        setProxyMode(false);
        toast.success("代理已禁用并保存");
      } else {
        // Apply proxy to runtime
        if (effectiveHost && effectivePort) {
          await testProxy(effectiveHost, effectivePort);
        }
        toast.success("代理已保存并应用");
      }

      // Mark as clean
      setCommittedMode(mode);
      setCommittedHost(effectiveHost);
      setCommittedPort(effectivePort);
    } catch (err: any) {
      toast.error(`保存失败: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const manualDisabled = mode !== "manual";

  return (
    <div className="section-card">
      <div className="section-title">代理</div>
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input type="radio" name="proxyMode" checked={mode === "none"} onChange={() => handleModeChange("none")} className="size-3" />
          无代理
        </label>
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input type="radio" name="proxyMode" checked={mode === "manual"} onChange={() => handleModeChange("manual")} className="size-3" />
          手动代理
        </label>
        <label className="flex items-center gap-1 text-xs cursor-pointer">
          <input type="radio" name="proxyMode" checked={mode === "system"} onChange={() => handleModeChange("system")} className="size-3" />
          系统代理
        </label>
        <span className="text-xs text-gray-500">主机:</span>
        <input
          type="text"
          value={effectiveHost}
          onChange={(e) => { setH(e.target.value); }}
          disabled={manualDisabled}
          className="w-[100px]"
        />
        <span className="text-xs text-gray-500">端口:</span>
        <input
          type="text"
          inputMode="numeric"
          pattern="[0-9]*"
          value={effectivePort}
          onChange={(e) => {
            const v = e.target.value.replace(/\D/g, "");
            setP(v ? parseInt(v, 10) : 7890);
          }}
          disabled={manualDisabled}
          className="w-[60px]"
        />
        <button className="btn text-xs px-2" onClick={handleTest} disabled={mode === "none"}>
          测试
        </button>
        <button
          className="btn flex items-center gap-1 text-xs px-2"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={12} />
          {saving ? "..." : "保存"}
        </button>
      </div>
    </div>
  );
}
