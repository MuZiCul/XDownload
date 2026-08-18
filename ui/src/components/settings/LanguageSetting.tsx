import { saveLanguage } from "../../lib/bindings";
import { toast } from "sonner";
import { useI18n, setLang, type Lang } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

type Props = {
  lang: string;
  onChange: (lang: string) => void;
};

export default function LanguageSetting({ lang, onChange }: Props) {
  const { t } = useI18n();

  const handleChange = async (value: string) => {
    const next = value as Lang;
    onChange(next);
    try {
      await saveLanguage(next);
      // Apply immediately (no restart needed).
      setLang(next === "en" ? "en" : "zh");
      toast.success(
        t("lang.saved", {
          lang: next === "zh" ? t("lang.zh") : t("lang.en"),
        })
      );
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    }
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("lang.title")} tip={t("lang.hintImmediate")} />
      <div className="flex items-center gap-2">
        <select
          value={lang}
          onChange={(e) => handleChange(e.target.value)}
          className="text-xs"
        >
          <option value="zh">{t("lang.zh")}</option>
          <option value="en">{t("lang.en")}</option>
        </select>
      </div>
    </div>
  );
}
