import { useRef, useState } from "react";
import { mutateAndSaveSettings } from "../../lib/settingsPersist";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
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
  /** 下载时防止系统休眠（Windows）。 */
  keepAwake: boolean;
  /** yt-dlp --limit-rate 值（如 "1M"）；空串 = 不限速。 */
  rateLimit: string;
  onChange: (
    patch: Partial<
      Pick<AppSettings, "concurrency" | "retry_count" | "queue_persist" | "keep_awake" | "download_rate_limit">
    >
  ) => void;
};

/** 多任务设置卡片：并发数 / 失败重试 / 队列持久化 / 下载限速 / 下载时防休眠。
 *  更改即自动保存（全局串行「读-改-写」防跨卡片竞态）。 */
export default function MultiTaskSetting({
  concurrency,
  retryCount,
  queuePersist,
  keepAwake,
  rateLimit,
  onChange,
}: Props) {
  const { t } = useI18n();
  // 下拉框的当前模式：预设档位 or 自定义。独立的模式状态（而非由 rateLimit
  // 值推断），避免"自定义值恰好等于某预设档位"时模式错乱。
  const [mode, setMode] = useState<"preset" | "custom">(() =>
    RATE_LIMIT_PRESETS.some((p) => p.value === rateLimit) ? "preset" : "custom"
  );
  // 最新设置值：初始来自 props，之后每次 onChange 同步合并（不依赖异步 render），
  // 保证快速连续操作时异步保存回调读到的始终是最新值。
  const latest = useRef({ concurrency, retryCount, queuePersist, keepAwake, rateLimit, mode });
  latest.current = { concurrency, retryCount, queuePersist, keepAwake, rateLimit, mode };

  const persist = (
    patch: Partial<
      Pick<AppSettings, "concurrency" | "retry_count" | "queue_persist" | "keep_awake" | "download_rate_limit">
    >,
    modeOverride?: "preset" | "custom"
  ) => {
    // 先把本次变更同步合并进 latest（显式映射：AppSettings 字段名 → ref 内部字段名），
    // 避免 onChange 异步 render 前的空窗读到旧值。mode 用 modeOverride 覆盖，
    // 解决"setMode 异步、persist 同步执行"导致的非法值校验失效。
    Object.assign(latest.current, {
      concurrency: patch.concurrency ?? latest.current.concurrency,
      retryCount: patch.retry_count ?? latest.current.retryCount,
      queuePersist: patch.queue_persist ?? latest.current.queuePersist,
      keepAwake: patch.keep_awake ?? latest.current.keepAwake,
      rateLimit: patch.download_rate_limit ?? latest.current.rateLimit,
      mode: modeOverride ?? latest.current.mode,
    });
    const cur = { ...latest.current };
    // 自定义限速值校验：非法则提示且不落盘该字段（其余字段仍保存），避免脏值进配置。
    const rateValid = cur.mode !== "custom" || cur.rateLimit === "" || RATE_LIMIT_RE.test(cur.rateLimit);
    if (!rateValid) {
      toast.error(t("multitask.invalidRate"));
    }
    // 全局串行「读-改-写」，避免与其它设置卡片的并发保存互相覆盖。
    mutateAndSaveSettings((cfg) => {
      cfg.concurrency = cur.concurrency;
      cfg.retry_count = cur.retryCount;
      cfg.queue_persist = cur.queuePersist;
      cfg.keep_awake = cur.keepAwake;
      if (rateValid) cfg.download_rate_limit = cur.rateLimit || "";
    }).catch((err: any) => {
      toast.error(t("common.saveFail", { err }));
    });
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("multitask.title")} tip={t("multitask.hint")} />
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("multitask.concurrency")}
          <select
            value={concurrency}
            onChange={(e) => {
              const patch = { concurrency: Number(e.target.value) };
              onChange(patch);
              persist(patch);
            }}
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
            onChange={(e) => {
              const patch = { retry_count: Number(e.target.value) };
              onChange(patch);
              persist(patch);
            }}
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
            onClick={() => {
              const patch = { queue_persist: !queuePersist };
              onChange(patch);
              persist(patch);
            }}
            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${queuePersist ? "bg-blue-600" : "bg-zinc-300"}`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${queuePersist ? "translate-x-4" : "translate-x-0.5"}`}
            />
          </button>
        </label>
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("multitask.keepAwake")}
          <button
            type="button"
            role="switch"
            aria-checked={keepAwake}
            onClick={() => {
              const patch = { keep_awake: !keepAwake };
              onChange(patch);
              persist(patch);
            }}
            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${keepAwake ? "bg-blue-600" : "bg-zinc-300"}`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${keepAwake ? "translate-x-4" : "translate-x-0.5"}`}
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
                  const patch = { download_rate_limit: "2M" };
                  onChange(patch);
                  persist(patch, "custom");
                }
              } else {
                setMode("preset");
                const patch = { download_rate_limit: v };
                onChange(patch);
                persist(patch, "preset");
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
            onChange={(e) => {
              const patch = { download_rate_limit: sanitizeRateLimitInput(e.target.value) };
              onChange(patch);
              persist(patch, "custom");
            }}
            placeholder="2M"
            className="text-xs border rounded px-2 py-1 w-20"
          />
        )}
      </div>
    </div>
  );
}
