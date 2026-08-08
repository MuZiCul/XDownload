import { RefreshCw, X } from "lucide-react";
import { useI18n } from "../../lib/i18n";

export interface DuplicateItem {
  url: string;
  video_id: string | null;
}

type Props = {
  /** 已下载、等待用户逐条处理的链接列表。 */
  items: DuplicateItem[];
  /** 点击「重新下载」：入队后父组件从 items 移除该条。 */
  onRedownload: (item: DuplicateItem) => void;
  /** 点击「取消下载」：父组件从 items 移除该条（不入队）。 */
  onCancel: (item: DuplicateItem) => void;
};

/**
 * 批量下载「已下载」确认弹窗（拟态玻璃）。
 * 设计约束：不能通过任何方式主动关闭 —— 没有关闭按钮、遮罩点击无效、
 * 无 Esc 处理；只有当所有链接都被处理（重新下载/取消）完毕、items 为空时，
 * 由父组件卸载本弹窗。
 */
export default function DuplicateDownloadModal({
  items,
  onRedownload,
  onCancel,
}: Props) {
  const { t } = useI18n();

  return (
    <div className="dialog-overlay">
      <div
        className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-[480px] max-w-[92vw] bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/50 p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-1">
          <span className="text-sm font-semibold text-zinc-900">
            {t("batch.dupTitle")}
          </span>
          <span className="text-[11px] text-zinc-400">
            {items.length} / {items.length}
          </span>
        </div>
        <p className="text-xs text-zinc-500 mb-3 leading-relaxed">
          {t("batch.dupBody")}
        </p>

        <div className="max-h-[260px] overflow-y-auto divide-y divide-zinc-200/70">
          {items.map((item) => (
            <div key={item.url} className="flex items-center gap-2 py-2">
              <span className="flex-1 min-w-0 text-xs text-zinc-700 truncate">
                {item.url}
              </span>
              <button
                className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 shrink-0"
                onClick={() => onRedownload(item)}
              >
                <RefreshCw size={12} />
                {t("video.redownload")}
              </button>
              <button
                className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 shrink-0 text-red-600"
                onClick={() => onCancel(item)}
              >
                <X size={12} />
                {t("video.cancelDownload")}
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
