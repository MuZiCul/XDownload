import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { ScrollText, X, RefreshCw, ArrowDownToLine } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import { getLogs } from "../../lib/bindings";
import type { LogsData } from "../../lib/types";

// ---- 模块级常量/纯函数（与组件渲染状态无关，只创建一次） ----

// ANSI 转义码 → React CSS 属性（颜色值适配深色主题）。
const ANSI: Record<string, CSSProperties> = {
  "1": { fontWeight: 700 },
  "2": { opacity: 0.7 },
  "3": { fontStyle: "italic" },
  "4": { textDecoration: "underline" },
  "30": { color: "#94a3b8" },
  "31": { color: "#f87171" },
  "32": { color: "#4ade80" },
  "33": { color: "#fbbf24" },
  "34": { color: "#60a5fa" },
  "35": { color: "#e879f9" },
  "36": { color: "#22d3ee" },
  "37": { color: "#e2e8f0" },
  "90": { color: "#64748b" },
  "91": { color: "#f87171" },
  "92": { color: "#4ade80" },
  "93": { color: "#fbbf24" },
  "94": { color: "#60a5fa" },
  "95": { color: "#e879f9" },
  "96": { color: "#22d3ee" },
  "97": { color: "#f8fafc" },
};

// 日志等级权重（用于按等级筛选）。
const LEVEL_ORDER: Record<string, number> = { ERROR: 4, WARN: 3, INFO: 2, DEBUG: 1, TRACE: 0 };

/** 从日志行提取等级（ERROR/WARN/INFO/DEBUG/TRACE），无则 null。 */
function lineLevel(line: string): string | null {
  return line.match(/\b(ERROR|WARN|INFO|DEBUG|TRACE)\b/)?.[1] ?? null;
}

/** 按选中等级过滤日志行：未选等级或行等级不低于选中等级则通过。 */
function passLevel(line: string, level: string): boolean {
  if (!level) return true;
  const lv = lineLevel(line);
  return !lv || (LEVEL_ORDER[lv] ?? 0) >= (LEVEL_ORDER[level] ?? 0);
}

/** 把一行日志解析成 [{text, style}] tokens，供 React <span style> 原生渲染。 */
function parseAnsiLine(line: string): { text: string; style?: CSSProperties }[] {
  const tokens: { text: string; style?: CSSProperties }[] = [];
  const re = /\x1b\[([0-9;]*)m/g;
  let last = 0;
  let style: CSSProperties | undefined;
  let m: RegExpExecArray | null;
  while ((m = re.exec(line)) !== null) {
    if (m.index > last) {
      tokens.push({ text: line.slice(last, m.index), style });
    }
    const codes = m[1].split(";");
    const reset = m[1] === "" || codes.includes("0");
    if (reset) {
      style = undefined;
    } else {
      const merged: CSSProperties = { ...style };
      for (const c of codes) {
        if (c !== "0" && ANSI[c]) Object.assign(merged, ANSI[c]);
      }
      style = merged;
    }
    last = re.lastIndex;
  }
  if (last < line.length) {
    tokens.push({ text: line.slice(last), style });
  }
  return tokens;
}

/** 渲染一行日志：tokens 转 React 元素（ERROR 行无 ANSI 颜色时兜底标红）。 */
function renderLine(line: string) {
  const isError = lineLevel(line) === "ERROR";
  const tokens = parseAnsiLine(line);
  const hasColor = tokens.some((tk) => tk.style?.color);
  return (
    <>
      {tokens.map((tk, i) => (
        <span
          key={i}
          style={isError && !hasColor ? { color: "#f87171", ...tk.style } : tk.style}
        >
          {tk.text}
        </span>
      ))}
      {"\n"}
    </>
  );
}

/** 文件大小格式化。 */
function fmtSize(n: number): string {
  if (n >= 1048576) return (n / 1048576).toFixed(2) + " MB";
  if (n >= 1024) return (n / 1024).toFixed(1) + " KB";
  return n + " B";
}

/** 应用内日志查看页。深色等宽、级别筛选、ANSI 着色、2s 自动刷新。 */
export default function LogViewerPage({ onClose }: { onClose: () => void }) {
  const { t } = useI18n();
  const [data, setData] = useState<LogsData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [file, setFile] = useState<string>("");
  const [level, setLevel] = useState("");
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [following, setFollowing] = useState(true);
  const [loading, setLoading] = useState(false);
  const mainRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async (targetFile: string) => {
    setLoading(true);
    setError(null);
    try {
      const d = await getLogs(targetFile || null, 2000);
      setData(d);
      setFile((prev) => (prev === targetFile && d.file ? d.file : d.file || prev));
    } catch (e: any) {
      setError(String(e?.message ?? e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load(file);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 2s 自动刷新
  useEffect(() => {
    if (!autoRefresh) return;
    const id = setInterval(() => {
      if (document.visibilityState !== "hidden") load(file);
    }, 2000);
    return () => clearInterval(id);
  }, [autoRefresh, file, load]);

  // 跟随最新：滚动到底部时保持跟随
  useEffect(() => {
    if (following && mainRef.current) {
      mainRef.current.scrollTop = mainRef.current.scrollHeight;
    }
  }, [data, following]);

  const handleFileChange = (f: string) => {
    setFile(f);
    setFollowing(true);
    load(f);
  };

  const handleScroll = () => {
    const el = mainRef.current;
    if (!el) return;
    setFollowing(el.scrollHeight - el.scrollTop - el.clientHeight < 150);
  };

  const shownLines = (data?.tail ?? []).filter((line) => passLevel(line, level));


  return (
    <div className="log-viewer fixed inset-0 z-40 flex flex-col bg-[#0f172a] text-[#cbd5e1]">
      {/* 顶栏 */}
      <div className="flex items-center gap-3 px-4 py-2 bg-[#1e293b] border-b border-[#334155] flex-wrap">
        <div className="flex items-center gap-2 text-[#f8fafc]">
          <ScrollText size={15} />
          <span className="text-sm font-semibold">{t("logs.title")}</span>
        </div>
        <select
          className="bg-[#0f172a] text-[#e2e8f0] border border-[#475569] rounded-md px-2 py-1 text-xs max-w-[300px] font-mono"
          value={file}
          onChange={(e) => handleFileChange(e.target.value)}
          title={t("logs.fileTitle")}
        >
          {(data?.files ?? []).map((f) => (
            <option key={f} value={f}>
              {f.replace(/^xdownload\.log\./, "")}
            </option>
          ))}
        </select>
        <select
          className="bg-[#0f172a] text-[#e2e8f0] border border-[#475569] rounded-md px-2 py-1 text-xs font-mono"
          value={level}
          onChange={(e) => setLevel(e.target.value)}
          title={t("logs.levelTitle")}
        >
          <option value="">{t("logs.levelAll")}</option>
          <option value="ERROR">ERROR</option>
          <option value="WARN">WARN</option>
          <option value="INFO">INFO</option>
          <option value="DEBUG">DEBUG</option>
          <option value="TRACE">TRACE</option>
        </select>
        <label className="flex items-center gap-1 text-xs text-[#94a3b8] cursor-pointer">
          <input
            type="checkbox"
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
          />
          {t("logs.autoRefresh")}
        </label>
        <button
          className="flex items-center gap-1 bg-[#2563eb] hover:bg-[#1d4ed8] text-white rounded-md px-3 py-1 text-xs"
          onClick={() => load(file)}
          title={t("logs.refresh")}
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : ""} />
          {t("logs.refresh")}
        </button>
        <button
          className="flex items-center gap-1 bg-transparent hover:bg-[#334155] text-[#94a3b8] rounded-md px-3 py-1 text-xs"
          onClick={() => {
            setFollowing(true);
            mainRef.current && (mainRef.current.scrollTop = mainRef.current.scrollHeight);
          }}
          title={t("logs.toLatest")}
        >
          <ArrowDownToLine size={12} />
          {t("logs.toLatest")}
        </button>
        <span className="ml-auto text-[11px] text-[#64748b] font-mono">
          {data?.file
            ? `${data.file.replace(/^xdownload\.log\./, "")} · ${fmtSize(data.size || 0)} · ${t("logs.shown", { n: shownLines.length, total: data.lines })}`
            : ""}
        </span>

        <button
          className="p-1 rounded-full text-[#94a3b8] hover:text-[#f8fafc] hover:bg-[#334155]"
          onClick={onClose}
          title={t("common.close")}
        >
          <X size={16} />
        </button>
      </div>

      {/* 日志区 */}
      <div
        ref={mainRef}
        onScroll={handleScroll}
        className="flex-1 overflow-auto px-4 py-2 font-mono text-[13px] leading-relaxed"
      >
        {error ? (
          <div className="text-red-400 py-8 text-center text-xs">{t("logs.loadError", { err: error })}</div>
        ) : !data || shownLines.length === 0 ? (
          <div className="text-[#64748b] py-8 text-center text-xs">
            {data?.file ? t("logs.empty") : t("logs.noFiles")}
          </div>
        ) : (
          <pre className="log-content whitespace-pre-wrap break-all text-[#cbd5e1]">
            {shownLines.map((line, i) => (
              <span key={i}>{renderLine(line)}</span>
            ))}
          </pre>
        )}
      </div>
    </div>
  );
}
