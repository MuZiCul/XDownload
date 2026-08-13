import { useEffect, useRef, useState } from "react";
import {
  useBookmarkSync,
  confirmBookmarkSelection,
  dismissBookmarkSync,
} from "../../lib/bookmarkSync";
import { Download, Loader2, X } from "lucide-react";
import { useI18n } from "../../lib/i18n";

/**
 * 书签同步全局模态：覆盖同步 → 确认 → 错误三个阶段。
 *
 * - syncing：spinner + 后端进度步骤，**无法关闭**（无关闭按钮、遮罩点击
 *   无效、无 Esc），切 tab 仍置顶显示——同步状态不因组件卸载而丢失。
 * - preview：勾选要入队的视频（默认勾选未下载），确认/取消后关闭。
 * - error：失败信息（queryId 失效引导等），可关闭。
 */
export default function BookmarkSyncModal() {
  const { t } = useI18n();
  const { phase, step, preview, error } = useBookmarkSync();

  if (phase === "idle") return null;
  return (
    <div
      className="dialog-overlay"
      onClick={phase === "preview" || phase === "error" ? dismissBookmarkSync : undefined}
    >
      <div
        className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[80] w-[540px] max-w-[92vw] bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/50 p-5"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        {phase === "syncing" && <SyncingStep step={step} />}
        {phase === "preview" && preview && <PreviewStep />}
        {phase === "error" && error && <ErrorStep />}
      </div>
    </div>
  );
}

function SyncingStep({ step }: { step: string | null }) {
  const { t } = useI18n();
  const labels: Record<string, string> = {
    cookies: t("bookmarks.step.cookies"),
    fetch: t("bookmarks.step.fetch"),
    persist: t("bookmarks.step.persist"),
    diff: t("bookmarks.step.diff"),
  };
  const active = step ? labels[step] ?? "" : "";
  return (
    <div className="py-6 flex flex-col items-center gap-4">
      <Loader2 size={28} className="text-blue-500 animate-spin" />
      <div className="text-center">
        <div className="text-sm font-medium text-zinc-800">
          {t("bookmarks.syncingTitle")}
        </div>
        <div className="text-xs text-zinc-500 mt-1.5 h-4">
          {active || t("bookmarks.step.prepare")}
        </div>
      </div>
    </div>
  );
}

function PreviewStep() {
  const { t } = useI18n();
  const { preview } = useBookmarkSync();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const selectAllRef = useRef<HTMLInputElement>(null);
  const items = preview?.video_items ?? [];

  // 打开时默认勾选未下载的书签；preview 变化时重置。
  useEffect(() => {
    setSelected(new Set(items.filter((it) => !it.downloaded).map((it) => it.tweet_id)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preview]);

  // 部分勾选时全选框显示半选横线。
  useEffect(() => {
    if (selectAllRef.current) {
      const total = items.length;
      const count = selected.size;
      selectAllRef.current.indeterminate = total > 0 && count > 0 && count < total;
    }
  }, [selected, items]);

  const toggleItem = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleAll = () => {
    if (selected.size === items.length) setSelected(new Set());
    else setSelected(new Set(items.map((it) => it.tweet_id)));
  };

  const handleConfirm = async () => {
    if (confirming) return;
    setConfirming(true);
    // 成功时 modal 关闭（phase→idle，组件卸载）；失败时保持打开并恢复按钮。
    await confirmBookmarkSelection(
      items.filter((it) => selected.has(it.tweet_id))
    );
    setConfirming(false);
  };

  return (
    <>
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-semibold text-zinc-900">
          {t("bookmarks.previewTitle")}
        </span>
        <button
          className="text-zinc-400 hover:text-zinc-600 transition-colors"
          onClick={dismissBookmarkSync}
        >
          <X size={16} />
        </button>
      </div>
      <p className="text-xs text-zinc-500 mb-3 leading-relaxed">
        {t("bookmarks.found", {
          total: preview?.total ?? 0,
          newCount: preview?.new_count ?? 0,
        })}
      </p>
      <div className="max-h-[260px] overflow-y-auto space-y-1.5 mb-4 pr-1">
        {items.map((item) => {
          const checked = selected.has(item.tweet_id);
          return (
            <div
              key={item.tweet_id}
              className={`flex items-start gap-2.5 px-3 py-2 rounded-lg border transition-colors ${
                item.downloaded
                  ? "bg-zinc-100/80 border-zinc-200/70"
                  : checked
                    ? "bg-blue-50/60 border-blue-400/60"
                    : "bg-white/70 border-zinc-200/70 hover:border-blue-400/60"
              }`}
            >
              <input
                type="checkbox"
                checked={checked}
                onChange={() => toggleItem(item.tweet_id)}
                className="mt-1 accent-blue-500 shrink-0"
              />
              <a
                href={item.url}
                target="_blank"
                rel="noopener noreferrer"
                className="block flex-1 min-w-0"
              >
                <div className="text-xs font-medium text-zinc-800 line-clamp-2">
                  {item.text}
                </div>
                <div className="text-[11px] text-zinc-500 mt-0.5">
                  @{item.handle} · {item.author_name}
                </div>
              </a>
              {item.downloaded && (
                <span className="shrink-0 text-[10px] text-zinc-500 bg-zinc-200/80 rounded px-1.5 py-0.5 mt-0.5">
                  {t("bookmarks.downloaded")}
                </span>
              )}
            </div>
          );
        })}
      </div>
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-1.5 text-xs text-zinc-500 cursor-pointer select-none">
            <input
              type="checkbox"
              ref={selectAllRef}
              checked={items.length > 0 && selected.size === items.length}
              onChange={toggleAll}
              className="accent-blue-500"
            />
            {t("bookmarks.selectAll")}
          </label>
          <span className="text-xs text-zinc-500">
            {t("bookmarks.selectedCount", { count: selected.size })}
          </span>
        </div>
        <div className="flex gap-2">
          <button
            className="btn flex items-center gap-1"
            onClick={dismissBookmarkSync}
            disabled={confirming}
          >
            {t("bookmarks.cancel")}
          </button>
          <button
            className="btn flex items-center gap-1"
            onClick={handleConfirm}
            disabled={confirming || selected.size === 0}
          >
            <Download size={13} />
            {confirming
              ? t("bookmarks.enqueueing")
              : t("bookmarks.enqueue", { count: selected.size })}
          </button>
        </div>
      </div>
    </>
  );
}

function ErrorStep() {
  const { t } = useI18n();
  const { error } = useBookmarkSync();
  if (!error) return null;
  return (
    <>
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-semibold text-red-600">
          {t("bookmarks.syncFailTitle")}
        </span>
        <button
          className="text-zinc-400 hover:text-zinc-600 transition-colors"
          onClick={dismissBookmarkSync}
        >
          <X size={16} />
        </button>
      </div>
      {error.queryId && (
        <div className="text-xs text-zinc-600 bg-red-50/70 border border-red-100 rounded-lg px-3 py-2 mb-2 leading-relaxed">
          {t("bookmarks.syncFailQueryId")}
        </div>
      )}
      <pre className="text-xs text-zinc-500 bg-zinc-50 rounded-lg p-3 overflow-auto max-h-40 whitespace-pre-wrap break-words">
        {error.msg}
      </pre>
      <div className="flex justify-end mt-4">
        <button className="btn" onClick={dismissBookmarkSync}>
          {t("bookmarks.cancel")}
        </button>
      </div>
    </>
  );
}
