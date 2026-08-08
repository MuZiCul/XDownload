import { useState } from "react";
import { saveSettings, loadSettings } from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save } from "lucide-react";
import { useI18n } from "../../lib/i18n";

type Props = {
  concurrency: number;
  retryCount: number;
  queuePersist: boolean;
  onChange: (
    patch: Partial<Pick<AppSettings, "concurrency" | "retry_count" | "queue_persist">>
  ) => void;
};

/** 多任务设置卡片：并发数 / 失败重试 / 队列持久化（三项同一卡片，单独保存）。 */
export default function MultiTaskSetting({
  concurrency,
  retryCount,
  queuePersist,
  onChange,
}: Props) {
  const { t } = useI18n();
  const [saving, setSaving] = useState(false);
  const [committed, setCommitted] = useState({ concurrency, retryCount, queuePersist });
  const changed =
    concurrency !== committed.concurrency ||
    retryCount !== committed.retryCount ||
    queuePersist !== committed.queuePersist;

  const handleSave = async () => {
    setSaving(true);
    try {
      const cfg: AppSettings = await loadSettings();
      cfg.concurrency = concurrency;
      cfg.retry_count = retryCount;
      cfg.queue_persist = queuePersist;
      await saveSettings(cfg);
      setCommitted({ concurrency, retryCount, queuePersist });
      toast.success(t("multitask.saved"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <div className="section-title">{t("multitask.title")}</div>
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("multitask.concurrency")}
          <select
            value={concurrency}
            onChange={(e) => onChange({ concurrency: Number(e.target.value) })}
            className="text-xs"
          >
            {[1, 2, 3].map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("multitask.retry")}
          <select
            value={retryCount}
            onChange={(e) => onChange({ retry_count: Number(e.target.value) })}
            className="text-xs"
          >
            {[0, 1, 2, 3, 4, 5].map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("multitask.persist")}
          <input
            type="checkbox"
            checked={queuePersist}
            onChange={(e) => onChange({ queue_persist: e.target.checked })}
            className="accent-blue-600"
          />
        </label>
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={13} />
          {saving ? t("common.saving") : t("common.save")}
        </button>
      </div>
      <div className="text-[11px] text-zinc-400 mt-2">{t("multitask.hint")}</div>
    </div>
  );
}
