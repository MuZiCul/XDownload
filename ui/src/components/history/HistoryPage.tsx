import { useEffect, useState } from "react";
import { FolderOpen, History as HistoryIcon, Trash2 } from "lucide-react";
import {
  listDownloadHistory,
  deleteDownloadHistory,
  clearDownloadHistory,
  openDownloadPath,
} from "../../lib/bindings";
import type { DownloadHistoryItem } from "../../lib/types";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";

function formatDateTime(ts: number): string {
  if (!ts) return "—";
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export default function HistoryPage() {
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
              <div key={item.id} className="flex items-center gap-3 py-2.5">
                <div className="flex-1 min-w-0">
                  <p className="text-[13px] font-medium text-zinc-800 truncate">
                    {item.title || item.id}
                  </p>
                  <p className="text-[11px] text-zinc-400 flex items-center gap-2">
                    {formatDateTime(item.downloaded_at)}
                    {!item.file_exists && (
                      <span className="text-red-500">{t("history.fileDeleted")}</span>
                    )}
                  </p>
                </div>
                <button
                  className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 shrink-0"
                  onClick={() => handleOpen(item.id)}
                  disabled={!item.file_exists}
                  title={t("video.openPath")}
                >
                  <FolderOpen size={12} />
                  {t("history.open")}
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
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
