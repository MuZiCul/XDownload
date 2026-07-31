import { useState, useEffect, useRef } from "react";
import { useToolStatus } from "../../hooks/useToolStatus";
import { pingGoogle, cancelBootstrapDownload } from "../../lib/bindings";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Ban, HelpCircle } from "lucide-react";

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
  const { ytStatus, ffStatus, download } = useToolStatus();
  const [phase, setPhase] = useState<Phase>(null);
  const [progress, setProgress] = useState(0);
  const unlistenRef = useRef<UnlistenFn | null>(null);

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

  return (
    <div className="section-card">
      <div className="section-title">Tools</div>
      <div className="flex items-center gap-2 flex-wrap">
        <button
          className="btn"
          disabled={ytStatus.available || phase !== null}
          onClick={() => handleDownload("yt-dlp")}
        >
          yt-dlp:{" "}
          {ytStatus.available
            ? "Latest"
            : phase?.kind === "downloading" && (phase as any).tool === "yt-dlp"
              ? "..."
              : "Download"}
        </button>
        <button
          className="btn"
          disabled={ffStatus.available || phase !== null}
          onClick={() => handleDownload("ffmpeg")}
        >
          ffmpeg:{" "}
          {ffStatus.available
            ? "Latest"
            : phase?.kind === "downloading" && (phase as any).tool === "ffmpeg"
              ? "..."
              : "Download"}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={() => setPhase({ kind: "guide" })}
        >
          <HelpCircle size={14} />
          下载指南
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
                  正在检测网络连接<Dots />
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
                  无法连接到下载站
                </p>
                <p className="text-xs text-gray-500 mb-5 leading-relaxed">
                  请检查网络连接或配置代理后重试。
                  <br />
                  是否仍然尝试下载 {phase.tool}？
                </p>
                <div className="flex items-center justify-center gap-3">
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 transition-colors"
                    onClick={() => startDownload(phase.tool)}
                  >
                    继续下载
                  </button>
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-gray-200 text-gray-600 font-medium hover:bg-gray-300 transition-colors"
                    onClick={() => setPhase(null)}
                  >
                    取消
                  </button>
                </div>
              </div>
            )}

            {/* ── Phase: extracting ffmpeg ── */}
            {phase.kind === "extracting" && (
              <div className="text-center">
                <p className="text-sm font-medium text-gray-700 mb-4">
                  正在解压 ffmpeg<Dots />
                </p>
                <div className="flex justify-center">
                  <div className="w-6 h-6 border-2 border-orange-400 border-t-transparent rounded-full animate-spin" />
                </div>
                <p className="text-[11px] text-gray-400 mt-3">
                  正在提取 ffmpeg.exe / ffprobe.exe / ffplay.exe
                </p>
              </div>
            )}

            {/* ── Phase: downloading ── */}
            {phase.kind === "downloading" && (
              <div className="text-center">
                <p className="text-sm font-medium text-gray-700 mb-5">
                  正在下载 {phase.tool}
                  {phase.tool === "ffmpeg"
                    ? "（约 80MB，解压后 ~150MB）"
                    : "（约 15MB）"}
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
                  {progress > 0 ? `${progress}%` : "正在连接..."}
                </p>

                <button
                  className="px-5 py-2 text-sm rounded-lg bg-red-50 text-red-600 font-medium hover:bg-red-100 transition-colors flex items-center gap-2 mx-auto"
                  onClick={handleCancelDownload}
                >
                  <Ban size={14} />
                  取消下载
                </button>
              </div>
            )}

            {/* ── Phase: done ── */}
            {phase.kind === "done" && (
              <div className="text-center">
                <div className="text-green-500 text-3xl mb-3">✓</div>
                <p className="text-sm font-medium text-gray-700">
                  {phase.tool} 下载完成
                </p>
              </div>
            )}

            {/* ── Phase: guide ── */}
            {phase.kind === "guide" && (
              <div>
                <p className="text-sm font-medium text-gray-800 mb-4 text-center">
                  下载指南
                </p>

                <div className="bg-amber-50 border border-amber-200 rounded-xl px-4 py-3 mb-5">
                  <p className="text-xs text-amber-700 leading-relaxed">
                    如遇下载慢或网络问题时，请先配置代理。
                    <br />
                    国内用户建议开启代理后再下载。
                    <br />
                    也可从下面地址下载后解压到根目录的bin中。
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
                      视频解析与下载引擎 · 约 15MB
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
                      音视频合并与转码 · 约 80MB（解压后 ~150MB）
                    </p>
                  </div>
                </div>

                <div className="mt-5 text-center">
                  <button
                    className="px-5 py-2 text-sm rounded-lg bg-gray-200 text-gray-600 font-medium hover:bg-gray-300 transition-colors"
                    onClick={() => setPhase(null)}
                  >
                    关闭
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
