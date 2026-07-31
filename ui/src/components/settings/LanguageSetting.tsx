import { useState } from "react";
import { saveLanguage } from "../../lib/bindings";
import { toast } from "sonner";
import { Save } from "lucide-react";

type Props = {
  lang: string;
  onChange: (lang: string) => void;
};

export default function LanguageSetting({ lang, onChange }: Props) {
  const [saving, setSaving] = useState(false);
  const [committed, setCommitted] = useState(lang);
  const changed = lang !== committed;

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveLanguage(lang);
      setCommitted(lang);
      toast.success(`语言已保存: ${lang === "zh" ? "中文" : "English"}（重启后完全生效）`);
    } catch (err: any) {
      toast.error(`保存失败: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <div className="section-title">语言 / Language</div>
      <div className="flex items-center gap-2">
        <select
          value={lang}
          onChange={(e) => onChange(e.target.value)}
          className="text-xs"
        >
          <option value="zh">中文</option>
          <option value="en">English</option>
        </select>
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={13} />
          {saving ? "..." : "保存"}
        </button>
        <span className="text-[11px] text-zinc-400">保存后重启生效</span>
      </div>
    </div>
  );
}
