import { useRef } from "react";
import { mutateAndSaveSettings } from "../../lib/settingsPersist";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

/** 并发分片数档位（yt-dlp --concurrent-fragments）。 */
export const HLS_CONCURRENT_PRESETS = [1, 2, 4, 6, 8, 12, 16];

/** 分片重试次数档位（yt-dlp --fragment-retries），0 = 不重试。 */
export const HLS_RETRY_PRESETS = [0, 3, 5, 10, 15, 20];

type Props = {
  /** yt-dlp --concurrent-fragments 值。 */
  concurrent: number;
  /** yt-dlp --fragment-retries 值。 */
  retries: number;
  onChange: (
    patch: Partial<
      Pick<AppSettings, "hls_concurrent_fragments" | "hls_fragment_retries">
    >
  ) => void;
};

/** HLS 下载设置卡片：分片并发数 / 分片重试次数。更改即自动保存。 */
export default function HlsSetting({ concurrent, retries, onChange }: Props) {
  const { t } = useI18n();
  // 最新设置值：初始来自 props，之后每次 onChange 同步合并（不依赖异步 render）。
  const latest = useRef({ concurrent, retries });
  latest.current = { concurrent, retries };

  const persist = (
    patch: Partial<Pick<AppSettings, "hls_concurrent_fragments" | "hls_fragment_retries">>
  ) => {
    // 显式映射：AppSettings 字段名 → ref 内部字段名。
    Object.assign(latest.current, {
      concurrent: patch.hls_concurrent_fragments ?? latest.current.concurrent,
      retries: patch.hls_fragment_retries ?? latest.current.retries,
    });
    const cur = { ...latest.current };
    // 全局串行「读-改-写」，避免与其它设置卡片的并发保存互相覆盖。
    mutateAndSaveSettings((cfg) => {
      cfg.hls_concurrent_fragments = cur.concurrent;
      cfg.hls_fragment_retries = cur.retries;
    }).catch((err: any) => {
      toast.error(t("common.saveFail", { err }));
    });
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("hls.title")} tip={t("hls.hint")} />
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("hls.concurrent")}
          <select
            value={concurrent}
            onChange={(e) => {
              const patch = { hls_concurrent_fragments: Number(e.target.value) };
              onChange(patch);
              persist(patch);
            }}
            className="text-xs"
          >
            {HLS_CONCURRENT_PRESETS.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("hls.retry")}
          <select
            value={retries}
            onChange={(e) => {
              const patch = { hls_fragment_retries: Number(e.target.value) };
              onChange(patch);
              persist(patch);
            }}
            className="text-xs"
          >
            {HLS_RETRY_PRESETS.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
      </div>
    </div>
  );
}
