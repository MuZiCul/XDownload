import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { saveSettings, loadSettings } from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save } from "lucide-react";

type Props = {
  dir: string;
  onChange: (dir: string) => void;
};

export default function DirSetting({ dir, onChange }: Props) {
  const [saving, setSaving] = useState(false);
  const [committed, setCommitted] = useState(dir);
  const changed = dir !== committed;

  const handleBrowse = async () => {
    const selected = await open({ directory: true, defaultPath: dir });
    if (selected) onChange(selected);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const cfg: AppSettings = await loadSettings();
      cfg.download_dir = dir || undefined;
      await saveSettings(cfg);
      setCommitted(dir);
      toast.success("视频保存位置已保存");
    } catch (err: any) {
      toast.error(`保存失败: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="section-card">
      <div className="section-title">视频保存位置</div>
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={dir}
          onChange={(e) => onChange(e.target.value)}
          placeholder="downloads"
          className="flex-1"
        />
        <button className="btn" onClick={handleBrowse}>
          浏览
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleSave}
          disabled={saving || !changed}
        >
          <Save size={13} />
          {saving ? "保存中..." : "保存"}
        </button>
      </div>
    </div>
  );
}
