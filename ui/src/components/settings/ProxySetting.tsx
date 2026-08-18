import { useState, useEffect, useRef } from "react";
import { testProxy, setProxyMode, getProxyStatus, applyManualProxy } from "../../lib/bindings";
import { mutateAndSaveSettings } from "../../lib/settingsPersist";
import type { ProxyStatus, ProxyTestResult } from "../../lib/types";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

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
  const [testState, setTestState] = useState<"idle" | "testing" | "success" | "error">("idle");
  // 最新输入值 ref：host/port 输入后若同一次渲染内立刻切换 radio，
  // 闭包里的 h/p 仍是旧值，用 ref 保证读到最新输入（修复#3）。
  const hRef = useRef(h);
  const pRef = useRef(p);
  hRef.current = h;
  pRef.current = p;

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

  const effectiveHost = mode === "system" && sysProxyHost ? sysProxyHost : h;
  const effectivePort = mode === "system" && sysProxyPort ? sysProxyPort : p;

  useEffect(() => {
    if (host) {
      setMode("manual");
      setH(host);
    }
    if (port) {
      setP(port);
    }
    if (scheme) {
      setSc(scheme);
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
        onChange(status.host || "", status.port || 0);
      } else if (status.enabled && !status.from_system) {
        setMode("manual");
        if (status.host) {
          setH(status.host);
          setP(status.port);
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

  // 更改即自动保存：把当前代理写入 settings.json 并应用到运行时。
  const persistProxy = async (
    host: string,
    port: number,
    targetMode: "none" | "manual" | "system",
    schemeOverride?: string
  ) => {
    const s = schemeOverride ?? sc;
    // 手动模式统一校验（覆盖 host onBlur / port onBlur / scheme 切换 / radio 切换
    // 所有入口）：host 非空、端口 > 0，无效则不落盘并提示，避免脏值（如 port=0）写盘。
    if (targetMode === "manual") {
      if (!host.trim()) {
        toast.warning(t("proxy.hostRequired"));
        return;
      }
      if (!port || port <= 0) {
        toast.warning(t("proxy.portRequired"));
        return;
      }
    }
    try {
      // 全局串行「读-改-写」，避免与其它设置卡片的并发保存互相覆盖。
      await mutateAndSaveSettings((cfg) => {
        if (targetMode === "none") {
          cfg.proxy_host = undefined;
          cfg.proxy_port = undefined;
          cfg.proxy_scheme = undefined;
          // 修复#2：切「无」时同步关闭「代理下载」开关，避免读盘时旧值
          // tools_use_proxy=true 被短暂写回（最终会被 setProxyMode(false)
          // 覆盖，但多一次脏写且前后端状态短暂不一致）。
          cfg.tools_use_proxy = false;
        } else {
          cfg.proxy_host = host;
          cfg.proxy_port = port;
          cfg.proxy_scheme = s;
        }
      });
      if (targetMode === "none") {
        await setProxyMode(false);
      } else if (targetMode === "manual") {
        // 手动代理：强制应用到运行时（覆盖系统代理标记），确保即时生效。
        await applyManualProxy(host, port, s);
      } else {
        // system 模式：setProxyMode(true) 内部对系统代理走"重新启用"
        // （ProxyConfig::enable() 保留系统来源标记），修复关闭后重开无法恢复。
        await setProxyMode(true);
      }
      // Notify other pages (e.g. DownloadPage) to reload the latest config.
      window.dispatchEvent(new CustomEvent("config-applied"));
      // 通知 ToolsSetting 重新探测代理可用性并同步「代理下载」开关。
      window.dispatchEvent(new CustomEvent("proxy-changed"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    }
  };

  const handleModeChange = (newMode: "none" | "manual" | "system") => {
    setMode(newMode);

    if (newMode === "none") {
      // persistProxy 内部会调用 setProxyMode(false) 并同步 tools_use_proxy，
      // 这里不再重复调用（修复#2 避免两次运行时切换）。
      onChange("", 0);
      persistProxy("", 0, "none");
    } else if (newMode === "system") {
      // persistProxy 内部 system 分支会调用 setProxyMode(true)，这里不重复调用。
      if (sysProxyHost) {
        onChange(sysProxyHost, sysProxyPort);
        persistProxy(sysProxyHost, sysProxyPort, "system");
      } else {
        toast.warning(t("proxy.noSystem"));
      }
    } else {
      setProxyMode(true).catch(() => {});
      // 用 ref 读取最新输入值（修复#3：输入后同一 tick 切换 radio 不再用旧闭包值）。
      // 校验统一由 persistProxy 负责（host 非空 + 端口有效，无效则提示不保存）。
      const curH = hRef.current;
      const curP = pRef.current;
      onChange(curH, curP);
      persistProxy(curH, curP, "manual");
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

  const manualDisabled = mode !== "manual";

  return (
    <div className="section-card">
      <SectionTitle
        title={
          <>
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
          </>
        }
        tip={t("proxy.hint")}
      />
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
            const v = e.target.value;
            setSc(v);
            setTestState("idle");
            // 更改即自动保存（仅手动模式编辑生效；校验统一由 persistProxy 负责）。
            if (mode === "manual") persistProxy(hRef.current, pRef.current, "manual", v);
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
          onChange={(e) => {
            // 轻量过滤（方案 B）：只允许 host 合法字符（字母/数字/. - _），
            // 拒绝空格与特殊符号，兼容 IP / 域名 / 主机名。
            const raw = e.target.value;
            if (!/^[a-zA-Z0-9._-]*$/.test(raw)) return;
            setH(raw);
            hRef.current = raw;
            setTestState("idle");
          }}
          onBlur={() => {
            if (mode === "manual") persistProxy(hRef.current, pRef.current, "manual");
          }}
          disabled={manualDisabled}
          className="[field-sizing:content]"
        />
        <span className="text-xs text-gray-500">{t("proxy.port")}</span>
        <input
          type="text"
          inputMode="numeric"
          pattern="[0-9]*"
          value={effectivePort ? effectivePort : ""}
          onChange={(e) => {
            const raw = e.target.value;
            // 严格限制：含任何非数字字符直接拒绝该次输入（输入框保持原值），
            // 只允许数字；端口区间 1~65535，超出也拒绝。
            if (!/^\d*$/.test(raw)) return;
            if (raw !== "" && parseInt(raw, 10) > 65535) return;
            setP(raw ? parseInt(raw, 10) : 0);
            pRef.current = raw ? parseInt(raw, 10) : 0;
            setTestState("idle");
          }}
          onBlur={() => {
            // 修复#1：允许清空输入；onBlur 时端口为空/0 则提示且不保存，
            // 有有效端口才自动保存。
            if (mode !== "manual") return;
            const curP = pRef.current;
            if (!curP || curP <= 0) {
              toast.warning(t("proxy.portRequired"));
              return;
            }
            persistProxy(hRef.current, curP, "manual");
          }}
          disabled={manualDisabled}
          className="[field-sizing:content]"
        />
        <button className="btn text-xs px-2" onClick={handleTest} disabled={mode === "none"}>
          {t("proxy.test")}
        </button>
      </div>
    </div>
  );
}
