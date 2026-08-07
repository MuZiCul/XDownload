import { Ban, CheckCircle2, X, XCircle } from "lucide-react";
import {
  useDownloadStore,
  cancelDownloadGlobal,
  dismissDownloadResult,
} from "../../lib/downloadStore";
import { useI18n } from "../../lib/i18n";

/**
 * Compact download progress indicator rendered inside the status bar (after
 * the ffmpeg badge). Shows the current download / completion / error state on
 * every tab. Completed / error banners stay until dismissed by the user.
 */
export default function GlobalDownloadBar() {
  const { downloading, progress, error, completed } = useDownloadStore();
  const { t } = useI18n();

  if (!downloading && !completed && !error) return null;

  const isMerging = progress?.stage === "merge";
  const stageLabel = isMerging
    ? t("gbar.stageMerge")
    : progress?.stage === "audio"
      ? t("gbar.stageAudio")
      : progress?.stage === "video"
        ? t("gbar.stageVideo")
        : t("gbar.progressLabel");

  return (
    <>
      {downloading && (
        <div className="flex items-center gap-2">
          <span className="text-[10px] text-zinc-500 shrink-0 whitespace-nowrap">
            {stageLabel}
          </span>
          {isMerging ? (
            /* Merge / post-processing: indeterminate running bar only */
            <div className="relative w-40 h-1.5 bg-zinc-100 rounded-full overflow-hidden shrink-0">
              <div
                className="absolute inset-y-0 w-2/5 rounded-full animate-progress-run"
                style={{
                  background: "linear-gradient(90deg, #3b82f6, #6366f1)",
                  boxShadow: "0 0 6px rgba(99, 102, 241, 0.4)",
                }}
              />
            </div>
          ) : (
            <>
              <div className="relative w-40 h-1.5 bg-zinc-100 rounded-full overflow-hidden shrink-0">
                {/* Gradient fill */}
                <div
                  className="absolute inset-y-0 left-0 rounded-full transition-all duration-200 ease-out"
                  style={{
                    width: `${Math.max(progress?.percent ?? 0, 2)}%`,
                    background: "linear-gradient(90deg, #3b82f6, #6366f1)",
                    boxShadow: "0 0 6px rgba(99, 102, 241, 0.4)",
                  }}
                />
                {/* Shimmer highlight sweeping across the track */}
                <div
                  className="absolute inset-y-0 w-10 blur-[2px] animate-progress-shimmer"
                  style={{
                    background: "linear-gradient(90deg, #3b82f6, #6366f1)",
                  }}
                />
              </div>
              <span className="text-[10px] tabular-nums text-zinc-600 w-9 text-right shrink-0">
                {progress?.percent ?? 0}%
              </span>
              <span className="text-[10px] tabular-nums text-zinc-400 w-16 text-right shrink-0">
                {progress?.speed || "—"}
              </span>
            </>
          )}
          <button
            className="text-[10px] text-red-600 hover:text-red-700 font-medium flex items-center gap-0.5 px-1.5 py-0.5 rounded hover:bg-red-50 transition-colors shrink-0"
            onClick={() => cancelDownloadGlobal()}
          >
            <Ban size={11} />
            {t("gbar.cancel")}
          </button>
        </div>
      )}

      {!downloading && completed && !error && (
        <div className="flex items-center gap-1.5 text-[11px] text-emerald-600">
          <CheckCircle2 size={13} className="shrink-0" />
          <span className="font-medium whitespace-nowrap">{t("gbar.complete")}</span>
          <button
            className="p-0.5 rounded hover:bg-zinc-100 text-zinc-400 hover:text-zinc-600"
            onClick={() => dismissDownloadResult()}
            aria-label={t("gbar.close")}
          >
            <X size={12} />
          </button>
        </div>
      )}

      {!downloading && error && (
        <div className="flex items-center gap-1.5 text-[11px] text-red-600 max-w-[260px]">
          <XCircle size={13} className="shrink-0" />
          <span className="truncate">{t("gbar.failed", { msg: error })}</span>
          <button
            className="p-0.5 rounded hover:bg-zinc-100 text-zinc-400 hover:text-zinc-600"
            onClick={() => dismissDownloadResult()}
            aria-label={t("gbar.close")}
          >
            <X size={12} />
          </button>
        </div>
      )}
    </>
  );
}
