import { useEffect, useState } from "react";
import { FolderOpen, History as HistoryIcon, RefreshCw, Trash2 } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import appIcon from "../../assets/icon.png";
import {
  listDownloadHistory,
  deleteDownloadHistory,
  clearDownloadHistory,
  openDownloadPath,
} from "../../lib/bindings";
import type { DownloadHistoryItem } from "../../lib/types";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";
import { formatDuration, formatNumber, formatDateTime } from "../../lib/format";

type Props = {
  onRedownload: (item: DownloadHistoryItem) => void;
};

export default function HistoryPage({ onRedownload }: Props) {
  const { t } = useI18n();
  const [items, setItems] = useState<DownloadHistoryItem[]>([]);
  const [loading, setLoading] = useState(true);

  const load = () => {
    setLoading(true);
    listDownloadHistory()
      .then(setItems)
      .catch(() => {})
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    load();
  }, []);

  const handleOpen = (id: string) => {
    openDownloadPath(id).catch((e: any) =>
      toast.error(t("video.openPathFail", { err: e }))
    );
  };

  const handleRedownload = (item: DownloadHistoryItem) => {
    if (!item.url) {
      toast.warning(t("history.noUrl"));
      return;
    }
    onRedownload(item);
  };

  const handleOpenLink = (item: DownloadHistoryItem) => {
    if (!item.url) return;
    openUrl(item.url).catch((e: any) =>
      toast.error(t("video.openUrlFail", { err: e }))
    );
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteDownloadHistory(id);
      setItems((prev) => prev.filter((i) => i.id !== id));
    } catch (e: any) {
      toast.error(t("history.deleteFail", { err: e }));
    }
  };

  const handleClear = async () => {
    try {
      await clearDownloadHistory();
      setItems([]);
      toast.success(t("history.cleared"));
    } catch (e: any) {
      toast.error(t("history.clearFail", { err: e }));
    }
  };

  return (
    <div className="p-3 max-w-[900px] mx-auto">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <HistoryIcon size={15} className="text-zinc-500" />
          <span className="text-[13px] font-semibold text-zinc-800">
            {t("history.title")}
          </span>
          {!loading && items.length > 0 && (
            <span className="text-[11px] text-zinc-400">
              {t("history.count", { count: items.length })}
            </span>
          )}
        </div>
        {!loading && items.length > 0 && (
          <button
            className="btn btn-danger px-3 py-1 text-xs font-semibold flex items-center gap-1"
            onClick={handleClear}
          >
            <Trash2 size={12} />
            {t("history.clearAll")}
          </button>
        )}
      </div>

      <div className="section-card">
        {loading ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("common.loading")}
          </p>
        ) : items.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("history.empty")}
          </p>
        ) : (
          <div className="divide-y divide-zinc-100">
            {items.map((item) => (
              <div key={item.id} className="flex gap-3 py-3">
                {/* Cover thumbnail — falls back to the app icon when the
                    recorded URL is missing or fails to load. */}
                <CoverThumb src={item.thumbnail} />

                <div className="flex-1 min-w-0">
                  <p
                    onClick={() => handleOpenLink(item)}
                    title={item.url ? t("video.openInBrowser") : undefined}
                    className={`text-[13px] font-semibold leading-snug line-clamp-2 mb-2 ${
                      item.url
                        ? "cursor-pointer hover:text-blue-600 hover:underline"
                        : "text-zinc-900"
                    }`}
                  >
                    {item.title || item.id}
                  </p>

                  {/* Author / duration / views / likes — same grid as the
                      download page info card. Legacy records lack these and
                      show "—". */}
                  <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs mb-2">
                    <InfoRow label={t("video.author")} value={item.uploader || "—"} />
                    <InfoRow
                      label={t("video.duration")}
                      value={item.duration > 0 ? formatDuration(item.duration) : "—"}
                    />
                    <InfoRow
                      label={t("video.views")}
                      value={item.view_count > 0 ? formatNumber(item.view_count, t) : "—"}
                    />
                    <InfoRow
                      label={t("video.likes")}
                      value={item.like_count > 0 ? formatNumber(item.like_count, t) : "—"}
                    />
                  </div>

                  {/* Download-time badge on the left, action buttons on the
                      same row right-aligned (mirrors the download page) */}
                  <div className="flex items-center gap-2 flex-wrap">
                    <div className="inline-flex items-center gap-1 text-[11px] font-medium text-zinc-600 bg-zinc-50 border border-zinc-200 rounded-md px-2 py-1">
                      {t("history.downloadedAt")} · {formatDateTime(item.downloaded_at)}
                    </div>
                    {!item.file_exists && (
                      <span className="text-[11px] text-red-500">
                        {t("history.fileDeleted")}
                      </span>
                    )}
                    <div className="flex items-center gap-2 ml-auto">
                      <button
                        className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 shrink-0"
                        onClick={() => handleOpen(item.id)}
                        disabled={!item.file_exists}
                        title={t("video.openPath")}
                      >
                        <FolderOpen size={12} />
                        {t("video.openPath")}
                      </button>
                      <button
                        className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 shrink-0"
                        onClick={() => handleRedownload(item)}
                        title={t("video.redownload")}
                      >
                        <RefreshCw size={12} />
                        {t("video.redownload")}
                      </button>
                      <button
                        className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 shrink-0 text-red-600"
                        onClick={() => handleDelete(item.id)}
                        title={t("history.delete")}
                      >
                        <Trash2 size={12} />
                        {t("history.delete")}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
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

/** Cover thumbnail that falls back to the app icon when the recorded URL is
 *  missing or fails to load. */
function CoverThumb({ src }: { src: string | null }) {
  const [failed, setFailed] = useState(false);
  const showFallback = !src || failed;
  return (
    <div className="w-28 h-[72px] rounded-lg border border-zinc-200 bg-zinc-900 overflow-hidden shrink-0 flex items-center justify-center">
      {showFallback ? (
        <img src={appIcon} alt="app" className="w-12 h-12 object-contain opacity-90" />
      ) : (
        <img
          src={src}
          alt="thumbnail"
          onError={() => setFailed(true)}
          className="w-full h-full object-cover"
        />
      )}
    </div>
  );
}
