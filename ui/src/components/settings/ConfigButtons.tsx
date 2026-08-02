import { useState, useEffect } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  saveSettings,
  saveSettingsToPath,
  getConfigPath,
  loadSettings,
  loadSettingsFromPath,
  applyAndPersistSettings,
  applyDefaultConfig,
  openConfigDir,
  quitApp,
} from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save, Upload, Folder, FolderOpen, FolderCog, Power, X } from "lucide-react";

type Props = {
  settings: AppSettings;
  onApply: (settings: AppSettings) => void;
};

export default function ConfigButtons({ settings, onApply }: Props) {
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [showApplyDialog, setShowApplyDialog] = useState(false);
  const [configPath, setConfigPath] = useState("");

  useEffect(() => {
    getConfigPath().then(setConfigPath).catch(() => {});
  }, []);

  const configDir = configPath
    ? configPath.replace(/[/\\][^/\\]*$/, "")
    : undefined;

  // ── Save ──────────────────────────────────────────────────────

  const handleSave = async (useCustom: boolean) => {
    setShowSaveDialog(false);

    if (useCustom) {
      // Export to user-chosen file
      const filePath = await save({
        defaultPath: configDir ? `${configDir}/settings.json` : "settings.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!filePath) return;

      try {
        await saveSettingsToPath(settings, filePath);
        toast.success(`配置已导出\n路径: ${filePath}`);
      } catch (err: any) {
        toast.error(`保存失败: ${err}`);
      }
    } else {
      // Save to active config
      try {
        await saveSettings(settings);
        toast.success(`配置已保存\n路径: ${configPath}`);
      } catch (err: any) {
        toast.error(`保存失败: ${err}`);
      }
    }
  };

  // ── Apply ─────────────────────────────────────────────────────

  const handleApply = (useCustom: boolean) => {
    setShowApplyDialog(false);
    setTimeout(() => applyConfig(useCustom), 0);
  };

  const applyConfig = async (useCustom: boolean) => {
    try {
      if (useCustom) {
        // Import from external file → apply + persist to active config
        const filePath = await open({
          defaultPath: configDir,
          filters: [{ name: "JSON", extensions: ["json"] }],
          multiple: false,
        });
        if (!filePath) return;

        const loaded = await loadSettingsFromPath(filePath as string);
        await applyAndPersistSettings(loaded);
        onApply(loaded);
        window.dispatchEvent(new CustomEvent("config-applied"));
        toast.success(`配置已导入并持久化\n来源: ${filePath}`);
      } else {
        // Restore from default config → apply + persist to active config
        const defaults = await applyDefaultConfig();
        onApply(defaults);
        window.dispatchEvent(new CustomEvent("config-applied"));
        toast.success(`已恢复默认配置\n路径: ${configPath}`);
      }
    } catch (err: any) {
      toast.error(`应用失败: ${err}`);
    }
  };

  // ── Config dir / Quit ────────────────────────────────────────

  const handleOpenConfigDir = async () => {
    try {
      await openConfigDir();
      toast.success("已打开配置目录");
    } catch (err: any) {
      toast.error(`打开失败: ${err}`);
    }
  };

  const handleQuit = async () => {
    try {
      await quitApp();
    } catch (err: any) {
      toast.error(`退出失败: ${err}`);
    }
  };

  // ── Render ────────────────────────────────────────────────────

  return (
    <>
      <div className="flex items-center gap-2 mt-1">
        <button
          className="btn flex items-center gap-1"
          onClick={() => setShowSaveDialog(true)}
        >
          <Save size={13} />
          保存配置
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={() => setShowApplyDialog(true)}
        >
          <Upload size={13} />
          应用配置
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleOpenConfigDir}
          title="打开根目录下的 config 文件夹"
        >
          <FolderCog size={13} />
          配置目录
        </button>
        <button
          className="btn flex items-center gap-1 text-red-600 hover:text-red-700"
          onClick={handleQuit}
          title="清理进程并退出应用"
        >
          <Power size={13} />
          退出
        </button>
        {configPath && (
          <span className="text-[10px] text-zinc-400 ml-auto truncate max-w-[260px]">
            配置: {configPath}
          </span>
        )}
      </div>

      {showSaveDialog && (
        <div className="dialog-overlay" onClick={() => setShowSaveDialog(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-semibold text-zinc-900">选择保存位置</h3>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={() => setShowSaveDialog(false)}
              >
                <X size={16} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-4">
              默认目录保存到应用内配置，自定义目录导出到其他位置
            </p>
            <div className="flex flex-col gap-2">
              <button
                className="btn btn-primary w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleSave(false)}
              >
                <Folder size={15} />
                默认目录（应用内配置）
              </button>
              <button
                className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleSave(true)}
              >
                <FolderOpen size={15} />
                自定义目录（导出）
              </button>
            </div>
          </div>
        </div>
      )}

      {showApplyDialog && (
        <div className="dialog-overlay" onClick={() => setShowApplyDialog(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-semibold text-zinc-900">选择配置来源</h3>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={() => setShowApplyDialog(false)}
              >
                <X size={16} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-4">
              默认目录恢复出厂设置，自定义目录从外部文件导入
            </p>
            <div className="flex flex-col gap-2">
              <button
                className="btn btn-primary w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleApply(false)}
              >
                <Folder size={15} />
                恢复默认
              </button>
              <button
                className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleApply(true)}
              >
                <FolderOpen size={15} />
                自定义目录（导入）
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
