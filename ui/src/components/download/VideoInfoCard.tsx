import { Download, FolderOpen, Play, RefreshCw } from "lucide-react";
import { openUrl, openPath } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import type { VideoInfo } from "../../lib/types";
import { useI18n } from "../../lib/i18n";
import { formatDuration, formatNumber, formatDateTime } from "../../lib/format";
import CoverThumb from "../common/CoverThumb";

type Props = {
  info: VideoInfo | null;
  onDownload: () => void;
  onOpenPath: () => void;
  /** 当前视频已在下载队列中（排队/下载中/暂停）——按钮禁用、徽标显示下载中。 */
  inQueue?: boolean;
};

export default function VideoInfoCard({
  info,
  onDownload,
  onOpenPath,
  inQueue,
}: Props) {
  const { t } = useI18n();
  const openLink = info?.webpage_url || info?.url || null;
  const handleOpenLink = () => {
    if (!openLink) return;
    openUrl(openLink).catch((e: any) =>
      toast.error(t("video.openUrlFail", { err: e }))
    );
  };
  // 用系统默认播放器播放已下载文件。
  const handlePlay = () => {
    if (!info?.download_path) return;
    toast.info(t("video.openingPlayer"));
    openPath(info.download_path).catch((e: any) =>
      toast.error(t("video.playFail", { err: e }))
    );
  };
  return (
    <div className="section-card">
      <div className="section-title">{t("video.info")}</div>

      {/* Thumbnail + metadata grid */}
      <div className="flex gap-3">
        <CoverThumb
          src={info?.thumbnail ?? null}
          stretch
          boxClass="w-[206px] h-[115.75px]"
        />
        <div className="flex-1 min-w-0">
          <h3
            onClick={handleOpenLink}
            title={openLink ? t("video.openInBrowser") : undefined}
            className={`text-[13px] font-semibold leading-snug line-clamp-2 mb-2 ${
              openLink
                ? "cursor-pointer hover:text-blue-600 hover:underline"
                : "text-zinc-900"
            }`}
          >
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

          {/* Badge (text only) + actions, right-aligned */}
          <div className="mt-2 flex items-center gap-2 flex-wrap">
            {info &&
              (inQueue ? (
                <div className="inline-flex items-center gap-1 text-[11px] font-medium text-blue-700 bg-blue-50 border border-blue-200 rounded-md px-2 py-1">
                  {t("batch.downloading")}
                </div>
              ) : info.downloaded ? (
                <div className="inline-flex items-center gap-1 text-[11px] font-medium text-emerald-700 bg-emerald-50 border border-emerald-200 rounded-md px-2 py-1">
                  {t("video.downloaded")}
                  {info.downloaded_at
                    ? ` · ${formatDateTime(info.downloaded_at)}`
                    : ""}
                </div>
              ) : (
                <div className="inline-flex items-center gap-1 text-[11px] font-medium text-zinc-500 bg-zinc-50 border border-zinc-200 rounded-md px-2 py-1">
                  {t("video.notDownloaded")}
                </div>
              ))}

            <div className="flex items-center gap-2 ml-auto">
              {info && (
                <>
                  {!inQueue && info.downloaded && (
                    <>
                      <button
                        className="btn px-3 py-1 text-xs font-semibold flex items-center gap-1 shadow-sm !bg-emerald-50 !border-emerald-200 hover:!border-emerald-300 text-emerald-700"
                        onClick={handlePlay}
                        title={t("video.play")}
                      >
                        <Play size={13} />
                        {t("video.play")}
                      </button>
                      <button
                        className="btn px-3 py-1 text-xs font-semibold flex items-center gap-1 shadow-sm"
                        onClick={onOpenPath}
                      >
                        <FolderOpen size={13} />
                        {t("video.openPath")}
                      </button>
                    </>
                  )}
                  <button
                    className="btn btn-primary px-3 py-1 text-xs font-semibold flex items-center gap-1 shadow-sm"
                    onClick={onDownload}
                    disabled={inQueue}
                  >
                    {inQueue ? (
                      t("video.downloading")
                    ) : info.downloaded ? (
                      <>
                        <RefreshCw size={13} />
                        {t("video.redownload")}
                      </>
                    ) : (
                      <>
                        <Download size={13} />
                        {t("video.startDownload")}
                      </>
                    )}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      </div>
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
