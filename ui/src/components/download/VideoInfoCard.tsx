import { Ban, Download, FolderOpen, RefreshCw } from "lucide-react";
import type { VideoInfo } from "../../lib/types";
import { useI18n, type Lang } from "../../lib/i18n";

function formatDuration(seconds: number): string {
  if (!seconds || seconds <= 0) return "?";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function formatNumber(
  n: number,
  t: (key: string) => string
): string {
  if (n >= 100_000_000) return `${(n / 100_000_000).toFixed(1)}${t("num.billion")}`;
  if (n >= 10_000) return `${(n / 10_000).toFixed(1)}${t("num.tenThousand")}`;
  return n.toLocaleString();
}

function formatDateTime(ts: number): string {
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

type Props = {
  info: VideoInfo | null;
  downloading: boolean;
  confirming: boolean;
  onDownload: () => void;
  onConfirmDownload: () => void;
  onCancelConfirm: () => void;
  onCancelDownload: () => void;
  onOpenPath: () => void;
};

export default function VideoInfoCard({
  info,
  downloading,
  confirming,
  onDownload,
  onConfirmDownload,
  onCancelConfirm,
  onCancelDownload,
  onOpenPath,
}: Props) {
  const { t, lang } = useI18n();
  const timeSuffix = info?.downloaded_at
    ? lang === "zh"
      ? `（${formatDateTime(info.downloaded_at)}）`
      : ` (${formatDateTime(info.downloaded_at)})`
    : "";
  return (
    <div className="section-card">
      <div className="section-title">{t("video.info")}</div>

      {/* Thumbnail + metadata grid */}
      <div className="flex gap-3">
        {info?.thumbnail && (
          <img
            src={info.thumbnail}
            alt="thumbnail"
            className="w-28 h-[72px] object-cover rounded-lg border border-zinc-200 shrink-0"
          />
        )}
        <div className="flex-1 min-w-0">
          <h3 className="text-[13px] font-semibold text-zinc-900 leading-snug line-clamp-2 mb-2">
            {info?.title ?? "—"}
          </h3>
          <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs">
            <InfoRow label={t("video.author")} value={info?.uploader ?? "—"} />
            <InfoRow label={t("video.duration")} value={info ? formatDuration(info.duration) : "—"} />
            <InfoRow label={t("video.views")} value={info ? formatNumber(info.view_count, t) : "—"} />
            <InfoRow label={t("video.likes")} value={info ? formatNumber(info.like_count, t) : "—"} />
          </div>

          {info?.media_count && info.media_count > 1 && (
            <div className="mt-2 inline-flex items-center gap-1 text-[11px] font-medium text-blue-700 bg-blue-50 border border-blue-200 rounded-md px-2 py-0.5">
              {t("video.multimedia", { count: info.media_count })}
            </div>
          )}

          {/* Downloaded badge on the left, action buttons right-aligned */}
          <div className="mt-2 flex items-center gap-2 flex-wrap">
            {info?.downloaded && (
              <div className="inline-flex items-center gap-1 text-[11px] font-medium text-emerald-700 bg-emerald-50 border border-emerald-200 rounded-md px-2 py-1">
                {t("video.downloaded")}
                {info.downloaded_at
                  ? ` · ${formatDateTime(info.downloaded_at)}`
                  : ""}
              </div>
            )}

            <div className="flex items-center gap-2 ml-auto">
              {downloading ? (
                <button
                  className="btn btn-danger px-3 py-1 text-xs font-semibold flex items-center gap-1"
                  onClick={onCancelDownload}
                >
                  <Ban size={13} />
                  {t("video.cancelDownload")}
                </button>
              ) : (
                <>
                  {info?.downloaded && (
                    <button
                      className="btn px-3 py-1 text-xs font-semibold flex items-center gap-1 shadow-sm"
                      onClick={onOpenPath}
                    >
                      <FolderOpen size={13} />
                      {t("video.openPath")}
                    </button>
                  )}
                  <button
                    className="btn btn-primary px-3 py-1 text-xs font-semibold flex items-center gap-1 shadow-sm"
                    onClick={onDownload}
                    disabled={!info}
                  >
                    {info?.downloaded ? (
                      <RefreshCw size={13} />
                    ) : (
                      <Download size={13} />
                    )}
                    {info?.downloaded
                      ? t("video.redownload")
                      : t("video.startDownload")}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Repeat-download confirmation */}
      {confirming && (
        <div className="dialog-overlay" onClick={onCancelConfirm}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-sm font-semibold text-zinc-900 mb-2">
              {t("video.repeatTitle")}
            </h3>
            <p className="text-xs text-zinc-500 mb-4 leading-relaxed">
              {t("video.repeatBody", { time: timeSuffix })}
            </p>
            <div className="flex gap-2 justify-end">
              <button className="btn" onClick={onCancelConfirm}>
                {t("common.cancel")}
              </button>
              <button
                className="btn btn-primary flex items-center gap-1"
                onClick={onConfirmDownload}
              >
                <RefreshCw size={13} />
                {t("video.redownload")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="text-zinc-400 shrink-0">{label}</span>
      <span className="text-zinc-700 truncate">{value}</span>
    </div>
  );
}
