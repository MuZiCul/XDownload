import { useState } from "react";
import { saveSettings, loadSettings } from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save } from "lucide-react";
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

/** HLS 下载设置卡片：分片并发数 / 分片重试次数（独立保存）。 */
export default function HlsSetting({ concurrent, retries, onChange }: Props) {
  const { t } = useI18n();
  const [saving, setSaving] = useState(false);
  const [committed, setCommitted] = useState({ concurrent, retries });
  const changed =
    concurrent !== committed.concurrent || retries !== committed.retries;

  const handleSave = async () => {
    setSaving(true);
    try {
      const cfg: AppSettings = await loadSettings();
      cfg.hls_concurrent_fragments = concurrent;
      cfg.hls_fragment_retries = retries;
      await saveSettings(cfg);
      setCommitted({ concurrent, retries });
      toast.success(t("hls.saved"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("hls.title")} tip={t("hls.hint")} />
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
        <label className="flex items-center gap-2 text-xs text-zinc-600">
          {t("hls.concurrent")}
          <select
            value={concurrent}
            onChange={(e) =>
              onChange({ hls_concurrent_fragments: Number(e.target.value) })
            }
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
            onChange={(e) =>
              onChange({ hls_fragment_retries: Number(e.target.value) })
            }
            className="text-xs"
          >
            {HLS_RETRY_PRESETS.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
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
    </div>
  );
}
