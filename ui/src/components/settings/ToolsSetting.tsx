import { useState, useEffect, useRef } from "react";
import { useToolStatus } from "../../hooks/useToolStatus";
import {
  pingGoogle,
  cancelBootstrapDownload,
  openRootDir,
} from "../../lib/bindings";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { Ban, Download, FolderOpen, HelpCircle, RefreshCw } from "lucide-react";
import type { YtdlpUpdateResult, FfmpegUpdateResult } from "../../lib/bindings";
import { useI18n } from "../../lib/i18n";

type Phase =
  | { kind: "checking" }
  | { kind: "warning"; tool: "yt-dlp" | "ffmpeg" }
  | { kind: "downloading"; tool: "yt-dlp" | "ffmpeg" }
  | { kind: "extracting"; tool: "ffmpeg" }
  | { kind: "done"; tool: "yt-dlp" | "ffmpeg" }
  | { kind: "guide" }
  | null;

// Animated ellipsis dots
function Dots() {
  const [n, setN] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setN((n) => (n + 1) % 4), 400);
    return () => clearInterval(t);
  }, []);
  return <span>{".".repeat(n)}&nbsp;</span>;
}

export default function ToolsSetting() {
  const { t } = useI18n();
  const { ytStatus, ffStatus, hasYtUpdate, hasFfUpdate, refresh, download } = useToolStatus();
  const [phase, setPhase] = useState<Phase>(null);
  const [progress, setProgress] = useState(0);
  const [checking, setChecking] = useState(false);
  const [updateResult, setUpdateResult] = useState<{
    yt: YtdlpUpdateResult | null;
    ff: FfmpegUpdateResult | null;
  } | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const handleCheckUpdate = async () => {
    if (checking) return;
    setChecking(true);
    try {
      const res = await refresh();
      setUpdateResult({ yt: res.ytUp, ff: res.ffUp });
    } catch (err: any) {
      toast.error(t("tools.checkFail", { err }));
    } finally {
      setChecking(false);
    }
  };

  // Clean up listener on unmount
  useEffect(() => {
    return () => {
      unlistenRef.current?.();
    };
  }, []);

  const handleDownload = async (tool: "yt-dlp" | "ffmpeg") => {
    if (phase) return;

    // --- Phase 1: Check network ---
    setPhase({ kind: "checking" });
    setProgress(0);

    const googleOk = await pingGoogle();

    if (!googleOk) {
      setPhase({ kind: "warning", tool });
      return;
    }

    // --- Phase 2: Download ---
    await startDownload(tool);
  };

  const startDownload = async (tool: "yt-dlp" | "ffmpeg") => {
    setPhase({ kind: "downloading", tool });
    setProgress(0);

    const unlisten = await listen<any>("bootstrap-progress", (event) => {
      if (event.payload.tool === tool) {
        if (event.payload.stage === "extracting") {
          setPhase({ kind: "extracting", tool: "ffmpeg" });
        } else {
          setProgress(event.payload.percent);
        }
      }
    });
    unlistenRef.current = unlisten;

    try {
      await download(tool);
      setPhase({ kind: "done", tool });
      setTimeout(() => setPhase(null), 2000);
    } catch {
      setPhase(null);
    } finally {
      unlistenRef.current = null;
      (await unlisten)();
    }
  };

  const handleCancelDownload = async () => {
    await cancelBootstrapDownload();
    unlistenRef.current?.();
    unlistenRef.current = null;
    setPhase(null);
  };

  const handleOpenRootDir = async () => {
    try {
      await openRootDir();
      toast.success(t("tools.rootOpened"));
    } catch (err: any) {
      toast.error(t("common.openFail", { err }));
    }
  };

  const renderToolRow = (
    name: "yt-dlp" | "ffmpeg",
    info: YtdlpUpdateResult | FfmpegUpdateResult | null
  ) => {
    const notInstalled = !!info?.not_installed;
    const hasUpdate = !!info?.has_update;
    const latest = info?.latest_version ?? null;
    const local = info?.local_version ?? null;
    const error = info?.error ?? null;

    let statusText: string;
    let statusClass = "text-gray-400";
    if (!info) {
      statusText = t("tools.statusCheckFailed");
    } else if (notInstalled) {
      statusText = t("tools.statusNotInstalled");
    } else if (error && !latest) {
      statusText = error;
      statusClass = "text-red-500";
    } else {
      statusText = local ? t("tools.statusCurrent", { ver: local }) : t("tools.statusUnknown");
      if (latest && latest !== local) {
        statusText += t("tools.statusLatest", { ver: latest });
        statusClass = "text-amber-600";
      } else {
        statusClass = "text-green-600";
      }
    }

    const showAction = !!info && (notInstalled || hasUpdate);

    return (
      <div className="bg-gray-50 rounded-xl px-4 py-3 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="text-xs font-semibold text-gray-700">{name}</p>
          <p
            className={`text-[11px] mt-0.5 leading-snug break-all ${statusClass}`}
          >
            {statusText}
          </p>
        </div>
        <div className="shrink-0">
          {showAction ? (
            <button
              className="px-3.5 py-1.5 text-xs rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 transition-colors flex items-center gap-1"
              onClick={() => {
                setUpdateResult(null);
                handleDownload(name);
              }}
            >
              <Download size={13} />
              {notInstalled ? t("tools.downloadBtn") : t("tools.updateBtn")}
            </button>
          ) : info ? (
            <span className="text-[11px] text-green-600 font-medium">{t("tools.latest")}</span>
          ) : (
            <span className="text-[11px] text-gray-400">—</span>
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="section-card">
      <div className="section-title">Tools</div>
      <div className="flex items-center gap-2 flex-wrap">
        <button
          className="btn"
          disabled={(ytStatus.available && !hasYtUpdate) || phase !== null}
          onClick={() => handleDownload("yt-dlp")}
        >
          yt-dlp:{" "}
          {phase?.kind === "downloading" && (phase as any).tool === "yt-dlp"
            ? "..."
            : !ytStatus.available
              ? "Download"
              : hasYtUpdate
                ? "Update"
                : "Latest"}
        </button>
        <button
          className="btn"
          disabled={(ffStatus.available && !hasFfUpdate) || phase !== null}
          onClick={() => handleDownload("ffmpeg")}
        >
          ffmpeg:{" "}
          {phase?.kind === "downloading" && (phase as any).tool === "ffmpeg"
            ? "..."
            : !ffStatus.available
              ? "Download"
              : hasFfUpdate
                ? "Update"
                : "Latest"}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={() => setPhase({ kind: "guide" })}
        >
          <HelpCircle size={14} />
          {t("tools.guideTitle")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleCheckUpdate}
          disabled={checking || phase !== null}
        >
          <RefreshCw size={13} className={checking ? "animate-spin" : ""} />
          {checking ? t("tools.checking") : t("tools.checkUpdate")}
        </button>
      </div>

      {/* Glassmorphism modal — all phases */}
      {phase && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/30 backdrop-blur-sm" />

          <div className="relative z-10 bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl p-8 w-[400px] border border-white/40">
            {/* ── Phase: checking network ── */}
            {phase.kind === "checking" && (
              <div className="text-center">
                <p className="text-sm font-medium text-gray-700 mb-4">
                  {t("tools.checkingNetwork")}<Dots />
                </p>
                <div className="flex justify-center">
                  <div className="w-6 h-6 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
                </div>
              </div>
            )}

            {/* ── Phase: warning (github unreachable) ── */}
            {phase.kind === "warning" && (
              <div className="text-center">
                <div className="text-amber-500 text-3xl mb-3">!</div>
                <p className="text-sm font-medium text-gray-800 mb-1">
                  {t("tools.networkFail")}
                </p>
                <p className="text-xs text-gray-500 mb-5 leading-relaxed">
                  {t("tools.networkFailBody", { tool: phase.tool })}
                </p>
                <div className="flex items-center justify-center gap-3">
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 transition-colors"
                    onClick={() => startDownload(phase.tool)}
                  >
                    {t("tools.continueDownload")}
                  </button>
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-gray-200 text-gray-600 font-medium hover:bg-gray-300 transition-colors"
                    onClick={() => setPhase(null)}
                  >
                    {t("common.cancel")}
                  </button>
                </div>
              </div>
            )}

            {/* ── Phase: extracting ffmpeg ── */}
            {phase.kind === "extracting" && (
              <div className="text-center">
                <p className="text-sm font-medium text-gray-700 mb-4">
                  {t("tools.extracting")}<Dots />
                </p>
                <div className="flex justify-center">
                  <div className="w-6 h-6 border-2 border-orange-400 border-t-transparent rounded-full animate-spin" />
                </div>
                <p className="text-[11px] text-gray-400 mt-3">
                  {t("tools.extractingDetail")}
                </p>
              </div>
            )}

            {/* ── Phase: downloading ── */}
            {phase.kind === "downloading" && (
              <div className="text-center">
                <p className="text-sm font-medium text-gray-700 mb-5">
                  {t("tools.downloading", { tool: phase.tool })}
                  {phase.tool === "ffmpeg"
                    ? t("tools.ffmpegSize")
                    : t("tools.ytdlpSize")}
                </p>

                <div className="w-full bg-gray-200/60 rounded-full h-3 overflow-hidden mb-2">
                  <div
                    className="h-full rounded-full transition-all duration-300 ease-out"
                    style={{
                      width: `${Math.max(progress, 4)}%`,
                      background:
                        progress < 100
                          ? "linear-gradient(90deg, #3b82f6, #6366f1)"
                          : "#22c55e",
                    }}
                  />
                </div>

                <p className="text-xs text-gray-400 tabular-nums mb-5">
                  {progress > 0 ? `${progress}%` : t("tools.connecting")}
                </p>

                <button
                  className="px-5 py-2 text-sm rounded-lg bg-red-50 text-red-600 font-medium hover:bg-red-100 transition-colors flex items-center gap-2 mx-auto"
                  onClick={handleCancelDownload}
                >
                  <Ban size={14} />
                  {t("tools.cancelDownload")}
                </button>
              </div>
            )}

            {/* ── Phase: done ── */}
            {phase.kind === "done" && (
              <div className="text-center">
                <div className="text-green-500 text-3xl mb-3">✓</div>
                <p className="text-sm font-medium text-gray-700">
                  {t("tools.downloadDone", { tool: phase.tool })}
                </p>
              </div>
            )}

            {/* ── Phase: guide ── */}
            {phase.kind === "guide" && (
              <div>
                <p className="text-sm font-medium text-gray-800 mb-4 text-center">
                  {t("tools.guideTitle")}
                </p>

                <div className="bg-amber-50 border border-amber-200 rounded-xl px-4 py-3 mb-5">
                  <p className="text-xs text-amber-700 leading-relaxed">
                    {t("tools.guideTip")}
                  </p>
                </div>

                <div className="space-y-4">
                  <div className="bg-gray-50 rounded-xl px-4 py-3">
                    <p className="text-xs font-semibold text-gray-700 mb-1">
                      yt-dlp
                    </p>
                    <a
                      href="https://github.com/yt-dlp/yt-dlp"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-[11px] text-blue-500 hover:text-blue-600 hover:underline leading-relaxed break-all"
                    >
                      https://github.com/yt-dlp/yt-dlp
                    </a>
                    <p className="text-[10px] text-gray-400 mt-1">
                      {t("tools.ytdlpDesc")}
                    </p>
                  </div>

                  <div className="bg-gray-50 rounded-xl px-4 py-3">
                    <p className="text-xs font-semibold text-gray-700 mb-1">
                      ffmpeg
                    </p>
                    <a
                      href="https://ffmpeg.org/download.html"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-[11px] text-blue-500 hover:text-blue-600 hover:underline leading-relaxed break-all"
                    >
                      https://ffmpeg.org/download.html
                    </a>
                    <p className="text-[10px] text-gray-400 mt-1">
                      {t("tools.ffmpegDesc")}
                    </p>
                  </div>
                </div>

                <div className="mt-5 flex items-center justify-center gap-3">
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 transition-colors flex items-center gap-1.5"
                    onClick={handleOpenRootDir}
                  >
                    <FolderOpen size={14} />
                    {t("tools.rootDir")}
                  </button>
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-gray-200 text-gray-600 font-medium hover:bg-gray-300 transition-colors"
                    onClick={() => setPhase(null)}
                  >
                    {t("common.close")}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── Modal: update check result ── */}
      {updateResult && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/30 backdrop-blur-sm" />
          <div className="relative z-10 bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl p-8 w-[420px] border border-white/40">
            <p className="text-sm font-semibold text-gray-800 mb-4 text-center">
              {t("tools.checkResultTitle")}
            </p>
            <div className="space-y-3">
              {renderToolRow("yt-dlp", updateResult.yt)}
              {renderToolRow("ffmpeg", updateResult.ff)}
            </div>
            <div className="mt-5 flex items-center justify-center">
              <button
                className="px-5 py-2 text-sm rounded-lg bg-gray-200 text-gray-600 font-medium hover:bg-gray-300 transition-colors"
                onClick={() => setUpdateResult(null)}
              >
                {t("common.close")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
