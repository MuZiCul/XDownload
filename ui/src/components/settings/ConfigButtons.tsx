import { openLogsDir } from "../../lib/bindings";
import { toast } from "sonner";
import { Power, ScrollText, Eye, EyeOff } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import { usePrivacyMode, setPrivacyMode } from "../../lib/privacyMode";

/**
 * 设置页顶部工具按钮条：软件日志 / 隐私模式 / 退出。
 * （配置保存/应用/配置目录等能力已于 2026-08-18 整体移除，设置项均为更改即自动保存。）
 */
export default function ConfigButtons() {
  const { t } = useI18n();
  const privacyMode = usePrivacyMode();

  const handleOpenLogsDir = async () => {
    try {
      await openLogsDir();
      toast.success(t("config.logsOpened"));
    } catch (err: any) {
      toast.error(t("common.openFail", { err }));
    }
  };

  const handleQuit = () => {
    // 交由 App 统一处理退出确认（无任务也弹窗：最小化到托盘 / 退出）。
    window.dispatchEvent(
      new CustomEvent("quit-requested", { detail: { source: "settings" } })
    );
  };

  return (
    <div className="sticky top-0 z-20 bg-[#fafafa]/95 backdrop-blur -mx-3 px-3 py-2 -mt-3 flex items-center gap-2 flex-wrap">
      <button
        className="btn flex items-center gap-1"
        onClick={handleOpenLogsDir}
        title={t("config.logsTitle")}
      >
        <ScrollText size={13} />
        {t("config.logs")}
      </button>
      <button
        className={
          privacyMode
            ? "btn flex items-center gap-1 text-amber-600 hover:bg-amber-50 hover:border-amber-200 hover:text-amber-700"
            : "btn flex items-center gap-1 hover:bg-amber-50 hover:border-amber-200 hover:text-amber-600"
        }
        onClick={() => setPrivacyMode(!privacyMode)}
        title={privacyMode ? t("privacy.disable") : t("privacy.enable")}
      >
        {privacyMode ? <EyeOff size={13} /> : <Eye size={13} />}
        {privacyMode ? t("privacy.disable") : t("privacy.enable")}
      </button>
      <button
        className="btn flex items-center gap-1 text-red-600 hover:bg-red-50 hover:border-red-200 hover:text-red-700"
        onClick={handleQuit}
        title={t("config.quitTitle")}
      >
        <Power size={13} />
        {t("config.quit")}
      </button>
    </div>
  );
}
