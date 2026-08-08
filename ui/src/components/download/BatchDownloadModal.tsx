import { X, ListPlus } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";
import { extractLinks } from "../../lib/urlUtils";

type Props = {
  /** 弹窗打开时的初始输入内容。 */
  initialInput?: string;
  /** 把提取出的链接交给父组件入队。 */
  onAdd: (urls: string[]) => void;
  onClose: () => void;
};

/** 批量下载输入弹窗（拟态玻璃风格）——仅输入框 + 「全部加入队列」。 */
export default function BatchDownloadModal({
  initialInput,
  onAdd,
  onClose,
}: Props) {
  const { t } = useI18n();
  const [input, setInput] = useState(initialInput ?? "");

  const handleAddAll = () => {
    const extracted = extractLinks(input);
    if (extracted.length === 0) {
      toast.warning(t("batch.empty"));
      return;
    }
    onAdd(extracted);
    setInput("");
    onClose();
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-[460px] max-w-[92vw] bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/50 p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm font-semibold text-zinc-900">
            {t("batch.title")}
          </span>
          <button
            className="text-zinc-400 hover:text-zinc-600 transition-colors"
            onClick={onClose}
            title={t("common.close")}
          >
            <X size={16} />
          </button>
        </div>

        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={t("batch.placeholder")}
          rows={6}
          autoFocus
          className="w-full text-xs px-3 py-2 border border-zinc-300 rounded-xl outline-none focus:border-blue-500 resize-y"
        />
        <div className="mt-2 flex items-center gap-2">
          <button
            className="btn btn-primary flex items-center gap-1.5"
            onClick={handleAddAll}
          >
            <ListPlus size={13} />
            {t("batch.addAll")}
          </button>
        </div>
      </div>
    </div>
  );
}
