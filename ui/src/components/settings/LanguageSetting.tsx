import { useState } from "react";
import { saveLanguage } from "../../lib/bindings";
import { toast } from "sonner";
import { Save } from "lucide-react";
import { useI18n, setLang, type Lang } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

type Props = {
  lang: string;
  onChange: (lang: string) => void;
};

export default function LanguageSetting({ lang, onChange }: Props) {
  const { t } = useI18n();
  const [saving, setSaving] = useState(false);
  const [committed, setCommitted] = useState(lang);
  const changed = lang !== committed;

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveLanguage(lang);
      setCommitted(lang);
      // Apply immediately (no restart needed).
      setLang(lang === "en" ? "en" : "zh");
      toast.success(
        t("lang.saved", {
          lang: lang === "zh" ? t("lang.zh") : t("lang.en"),
        })
      );
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("lang.title")} tip={t("lang.hintImmediate")} />
      <div className="flex items-center gap-2">
        <select
          value={lang}
          onChange={(e) => onChange(e.target.value as Lang)}
          className="text-xs"
        >
          <option value="zh">{t("lang.zh")}</option>
          <option value="en">{t("lang.en")}</option>
        </select>
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={13} />
          {saving ? "..." : t("common.save")}
        </button>
      </div>
    </div>
  );
}
