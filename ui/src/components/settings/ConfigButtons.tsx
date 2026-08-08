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
  openLogsDir,
} from "../../lib/bindings";
import type { AppSettings } from "../../lib/types";
import { toast } from "sonner";
import { Save, Upload, Folder, FolderOpen, FolderCog, Power, ScrollText, X } from "lucide-react";
import { useI18n } from "../../lib/i18n";

type Props = {
  settings: AppSettings;
  onApply: (settings: AppSettings) => void;
};

export default function ConfigButtons({ settings, onApply }: Props) {
  const { t } = useI18n();
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
        toast.success(t("config.exported", { path: filePath }));
      } catch (err: any) {
        toast.error(t("common.saveFail", { err }));
      }
    } else {
      // Save to active config
      try {
        await saveSettings(settings);
        toast.success(t("config.saved", { path: configPath }));

        // Notify other pages (e.g. DownloadPage) to reload the latest config.
        window.dispatchEvent(new CustomEvent("config-applied"));
      } catch (err: any) {
        toast.error(t("common.saveFail", { err }));
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
        toast.success(t("config.imported", { path: filePath }));
      } else {
        // Restore from default config → apply + persist to active config
        const defaults = await applyDefaultConfig();
        onApply(defaults);
        window.dispatchEvent(new CustomEvent("config-applied"));
        toast.success(t("config.restored", { path: configPath }));
      }
    } catch (err: any) {
      toast.error(t("common.applyFail", { err }));
    }
  };

  // ── Config dir / Quit ────────────────────────────────────────

  const handleOpenConfigDir = async () => {
    try {
      await openConfigDir();
      toast.success(t("config.dirOpened"));
    } catch (err: any) {
      toast.error(t("common.openFail", { err }));
    }
  };

  const handleOpenLogsDir = async () => {
    try {
      await openLogsDir();
      toast.success(t("config.logsOpened"));
    } catch (err: any) {
      toast.error(t("common.openFail", { err }));
    }
  };

  const handleQuit = () => {
    // 交由 App 统一处理退出确认（有任务时弹窗，无任务直接退出）。
    window.dispatchEvent(
      new CustomEvent("quit-requested", { detail: { source: "settings" } })
    );
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
          {t("config.save")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={() => setShowApplyDialog(true)}
        >
          <Upload size={13} />
          {t("config.apply")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleOpenConfigDir}
          title={t("config.dirTitle")}
        >
          <FolderCog size={13} />
          {t("config.dir")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={handleOpenLogsDir}
          title={t("config.logsTitle")}
        >
          <ScrollText size={13} />
          {t("config.logs")}
        </button>
        <button
          className="btn flex items-center gap-1 text-red-600 hover:text-red-700"
          onClick={handleQuit}
          title={t("config.quitTitle")}
        >
          <Power size={13} />
          {t("config.quit")}
        </button>
        {configPath && (
          <span className="text-[10px] text-zinc-400 ml-auto truncate max-w-[260px]">
            {t("config.path", { path: configPath })}
          </span>
        )}
      </div>

      {showSaveDialog && (
        <div className="dialog-overlay" onClick={() => setShowSaveDialog(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-semibold text-zinc-900">
                {t("config.saveDialogTitle")}
              </h3>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={() => setShowSaveDialog(false)}
              >
                <X size={16} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-4">
              {t("config.saveDialogBody")}
            </p>
            <div className="flex flex-col gap-2">
              <button
                className="btn btn-primary w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleSave(false)}
              >
                <Folder size={15} />
                {t("config.saveDefault")}
              </button>
              <button
                className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleSave(true)}
              >
                <FolderOpen size={15} />
                {t("config.saveCustom")}
              </button>
            </div>
          </div>
        </div>
      )}

      {showApplyDialog && (
        <div className="dialog-overlay" onClick={() => setShowApplyDialog(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-semibold text-zinc-900">
                {t("config.applyDialogTitle")}
              </h3>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={() => setShowApplyDialog(false)}
              >
                <X size={16} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-4">
              {t("config.applyDialogBody")}
            </p>
            <div className="flex flex-col gap-2">
              <button
                className="btn btn-primary w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleApply(false)}
              >
                <Folder size={15} />
                {t("config.applyDefault")}
              </button>
              <button
                className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                onClick={() => handleApply(true)}
              >
                <FolderOpen size={15} />
                {t("config.applyCustom")}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
