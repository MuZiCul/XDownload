import { Power, ScrollText, Eye, EyeOff, BarChart3 } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import { usePrivacyMode, setPrivacyMode } from "../../lib/privacyMode";

/**
 * 设置页顶部工具按钮条：软件日志 / 隐私模式 / 统计 / 退出。
 * （配置保存/应用/配置目录等能力已于 2026-08-18 整体移除，设置项均为更改即自动保存。
 * 「软件日志」打开应用内日志页。）
 */
export default function ConfigButtons({
  onOpenLogs,
  onOpenStats,
}: {
  onOpenLogs?: () => void;
  onOpenStats?: () => void;
}) {
  const { t } = useI18n();
  const privacyMode = usePrivacyMode();

  const handleQuit = () => {
    // 交由 App 统一处理退出确认（无任务也弹窗：最小化到托盘 / 退出）。
    window.dispatchEvent(
      new CustomEvent("quit-requested", { detail: { source: "settings" } })
    );
  };

  return (
    <div className="sticky top-0 z-20 bg-[#fafafa]/95 backdrop-blur -mx-3 px-3 py-2 -mt-3 flex items-center gap-2 flex-wrap">
      {onOpenLogs && (
        <button
          className="btn flex items-center gap-1"
          onClick={onOpenLogs}
          title={t("config.logsTitle")}
        >
          <ScrollText size={13} />
          {t("config.logs")}
        </button>
      )}
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
      {onOpenStats && (
        <button
          className="btn flex items-center gap-1"
          onClick={onOpenStats}
          title={t("stats.openTitle")}
        >
          <BarChart3 size={13} />
          {t("stats.open")}
        </button>
      )}
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
