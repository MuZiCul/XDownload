import { useState } from "react";
import { saveSettings, loadSettings } from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

/** 预设限速档位（yt-dlp --limit-rate），从低到高 1M~100M 五档均分。
 *  "unlimited" 表示不限速。 */
export const RATE_LIMIT_PRESETS = [
  { label: "unlimited", value: "" },
  { label: "1M", value: "1M" },
  { label: "25M", value: "25M" },
  { label: "50M", value: "50M" },
  { label: "75M", value: "75M" },
  { label: "100M", value: "100M" },
];

/** 合法限速格式：数字（可带小数）+ 可选 K/M/G 单位（大小写均可），如 "1M"、"2.5M"、"500K"。 */
export const RATE_LIMIT_RE = /^\d+(\.\d+)?[KMGkmg]?$/;

/** 只保留限速输入允许的字符：数字、小数点、单位字母（K/M/G，大小写）。 */
export function sanitizeRateLimitInput(s: string): string {
  // eslint-disable-next-line no-control-regex
  return s.replace(/[^0-9.KMGkmg]/g, "");
}

type Props = {
  concurrency: number;
  retryCount: number;
  queuePersist: boolean;
  /** yt-dlp --limit-rate 值（如 "1M"）；空串 = 不限速。 */
  rateLimit: string;
  onChange: (
    patch: Partial<
      Pick<AppSettings, "concurrency" | "retry_count" | "queue_persist" | "download_rate_limit">
    >
  ) => void;
};

/** 多任务设置卡片：并发数 / 失败重试 / 队列持久化 / 下载限速（同一卡片，单独保存）。 */
export default function MultiTaskSetting({
  concurrency,
  retryCount,
  queuePersist,
  rateLimit,
  onChange,
}: Props) {
  const { t } = useI18n();
  const [saving, setSaving] = useState(false);
  // 下拉框的当前模式：预设档位 or 自定义。独立的模式状态（而非由 rateLimit
  // 值推断），避免"自定义值恰好等于某预设档位"时模式错乱。
  const [mode, setMode] = useState<"preset" | "custom">(() =>
    RATE_LIMIT_PRESETS.some((p) => p.value === rateLimit) ? "preset" : "custom"
  );
  const [committed, setCommitted] = useState({
    concurrency,
    retryCount,
    queuePersist,
    rateLimit,
  });
  const changed =
    concurrency !== committed.concurrency ||
    retryCount !== committed.retryCount ||
    queuePersist !== committed.queuePersist ||
    rateLimit !== committed.rateLimit;

  const handleSave = async () => {
    // 保存前校验自定义限速值（预设档位永远合法）；非法则提示并回滚，不落盘。
    if (mode === "custom" && rateLimit !== "" && !RATE_LIMIT_RE.test(rateLimit)) {
      toast.error(t("multitask.invalidRate"));
      onChange({ download_rate_limit: committed.rateLimit });
      return;
    }
    setSaving(true);
    try {
      const cfg: AppSettings = await loadSettings();
      cfg.concurrency = concurrency;
      cfg.retry_count = retryCount;
      cfg.queue_persist = queuePersist;
      cfg.download_rate_limit = rateLimit || "";
      await saveSettings(cfg);
      setCommitted({ concurrency, retryCount, queuePersist, rateLimit });
      toast.success(t("multitask.saved"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("multitask.title")} tip={t("multitask.hint")} />
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
          <button
            type="button"
            role="switch"
            aria-checked={queuePersist}
            onClick={() => onChange({ queue_persist: !queuePersist })}
            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${queuePersist ? "bg-blue-600" : "bg-zinc-300"}`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${queuePersist ? "translate-x-4" : "translate-x-0.5"}`}
            />
          </button>
        </label>
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("multitask.rateLimit")}
          <select
            value={mode === "custom" ? "custom" : rateLimit}
            onChange={(e) => {
              const v = e.target.value;
              if (v === "custom") {
                setMode("custom");
                // 进入自定义：若无当前值，给一个合法默认让输入框有内容可编辑。
                if (rateLimit === "" || RATE_LIMIT_PRESETS.some((p) => p.value === rateLimit)) {
                  onChange({ download_rate_limit: "2M" });
                }
              } else {
                setMode("preset");
                onChange({ download_rate_limit: v });
              }
            }}
            className="text-xs"
          >
            {RATE_LIMIT_PRESETS.map((p) => (
              <option key={p.value || "unlimited"} value={p.value}>
                {p.value === ""
                  ? t("multitask.unlimited")
                  : p.label}
              </option>
            ))}
            <option value="custom">{t("multitask.custom")}</option>
          </select>
        </label>
        {mode === "custom" && (
          <input
            type="text"
            value={rateLimit}
            onChange={(e) =>
              onChange({ download_rate_limit: sanitizeRateLimitInput(e.target.value) })
            }
            placeholder="2M"
            className="text-xs border rounded px-2 py-1 w-20"
          />
        )}
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={13} />
          {saving ? t("common.saving") : t("common.save")}
        </button>
      </div>
    </div>
  );
}
