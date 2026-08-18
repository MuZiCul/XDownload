import { open } from "@tauri-apps/plugin-dialog";
import { openDownloadDir } from "../../lib/bindings";
import { mutateAndSaveSettings } from "../../lib/settingsPersist";
import { toast } from "sonner";
import { Pencil, FolderOpen } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import SectionTitle from "./SectionTitle";

type Props = {
  dir: string;
  onChange: (dir: string) => void;
};

/**
 * 视频保存位置卡片（图式布局）：
 *   描述说明 + 路径（两行）      ✏️  📂
 * - ✏️ 点击弹出目录选择，选中后立即保存并写回 state。
 * - 📂 打开当前下载目录。
 * - 操作按钮组在右侧垂直居中，无标题 / 无大图标 / 无输入框。
 */
export default function DirSetting({ dir, onChange }: Props) {
  const { t } = useI18n();

  const handleBrowse = async () => {
    const selected = await open({ directory: true, defaultPath: dir });
    if (!selected) return;
    // 更改即自动保存（全局串行「读-改-写」，防跨卡片并发覆盖）。
    try {
      const target = selected as string;
      await mutateAndSaveSettings((cfg) => {
        cfg.download_dir = target;
      });
      onChange(target);
      toast.success(t("dir.saved"));
    } catch (err: any) {
      toast.error(t("common.saveFail", { err }));
    }
  };

  const handleOpen = async () => {
    try {
      await openDownloadDir();
    } catch (err: any) {
      toast.error(t("common.openFail", { err }));
    }
  };

  return (
    <div className="section-card">
      <SectionTitle title={t("dir.title")} tip={t("dir.tip")} />
      <div className="flex items-center justify-between gap-2">
        {/* 左侧：路径 */}
        <div className="min-w-0">
          <div className="text-xs text-zinc-500 truncate" title={dir}>
            {dir}
          </div>
        </div>

        {/* 右侧：操作按钮组（垂直居中） */}
        <div className="flex items-center gap-0.5 shrink-0">
          <button
            className="p-1.5 rounded-lg text-zinc-400 hover:bg-zinc-100 hover:text-zinc-900 transition-colors"
            onClick={handleBrowse}
            title={t("dir.edit")}
          >
            <Pencil size={14} />
          </button>
          <button
            className="p-1.5 rounded-lg text-zinc-400 hover:bg-zinc-100 hover:text-zinc-900 transition-colors"
            onClick={handleOpen}
            title={t("dir.open")}
          >
            <FolderOpen size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
