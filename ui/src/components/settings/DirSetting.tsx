import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { saveSettings, loadSettings, openDownloadDir } from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save, FolderOpen } from "lucide-react";
import { useI18n } from "../../lib/i18n";

type Props = {
  dir: string;
  onChange: (dir: string) => void;
};

export default function DirSetting({ dir, onChange }: Props) {
  const { t } = useI18n();
  const [saving, setSaving] = useState(false);
  const [committed, setCommitted] = useState(dir);
  const changed = dir !== committed;

  const handleBrowse = async () => {
    const selected = await open({ directory: true, defaultPath: dir });
    if (selected) onChange(selected);
  };

  const handleOpen = async () => {
    try {
      await openDownloadDir();
    } catch (err: any) {
      toast.error(t("common.openFail", { err }));
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const cfg: AppSettings = await loadSettings();
      cfg.download_dir = dir || undefined;
      await saveSettings(cfg);
      setCommitted(dir);
      toast.success(t("dir.saved"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <div className="section-title">{t("dir.title")}</div>
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={dir}
          onChange={(e) => onChange(e.target.value)}
          placeholder="downloads"
          className="flex-1"
        />
        <button className="btn" onClick={handleBrowse}>
          {t("dir.browse")}
        </button>
        <button className="btn flex items-center gap-1" onClick={handleOpen}>
          <FolderOpen size={13} />
          {t("dir.open")}
        </button>
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
