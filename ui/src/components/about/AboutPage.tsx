import { useState, useEffect } from "react";
import { Github, RefreshCw } from "lucide-react";
import { getVersion } from "../../lib/bindings";
import { check } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";

export default function AboutPage() {
  const { t } = useI18n();
  const [checking, setChecking] = useState(false);
  const [version, setVersion] = useState("");

  // 版本号动态获取（数据源：Cargo.toml）。
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  const handleCheckUpdate = async () => {
    if (checking) return;
    setChecking(true);

    try {
      const update = await check();

      if (update) {
        toast(t("about.newVersion", { ver: update.version }), {
          description: t("about.currentVersion", {
            ver: update.currentVersion,
          }),
          action: {
            label: t("about.goDownload"),
            onClick: () => {
              // 打开与启动检查一致的拟态玻璃更新窗（可下载更新/前往 GitHub）。
              window.dispatchEvent(
                new CustomEvent("open-update-modal", {
                  detail: {
                    version: update.version,
                    currentVersion: update.currentVersion,
                  },
                })
              );
            },
          },
          duration: 8000,
        });
      } else {
        toast.success(t("about.upToDate"));
      }
    } catch {
      toast.error(t("about.checkFail"));
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="h-full flex items-center justify-center py-10 px-6">
      <div className="w-full max-w-lg">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-gray-900 mb-1">XDownload</h1>

          {/* Version + check update button */}
          <div className="flex items-center justify-center gap-2 mb-2">
            <p className="text-sm text-gray-500">{version ? `v${version}` : ""}</p>
            <button
              className="inline-flex items-center gap-1 text-[11px] text-blue-500 hover:text-blue-600 hover:underline disabled:text-gray-300 disabled:no-underline transition-colors"
              onClick={handleCheckUpdate}
              disabled={checking}
            >
              <RefreshCw
                size={12}
                className={checking ? "animate-spin" : ""}
              />
              {checking ? t("about.checking") : t("about.checkUpdate")}
            </button>
          </div>

          <div className="h-5" />
          <p className="text-sm text-gray-600 leading-relaxed mb-1">
            {t("about.desc")}
          </p>
          <p className="text-xs text-gray-500 mb-3">By MuZiCul</p>
          <a
            href="https://github.com/MuZiCul/XDownload"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-sm text-[#0969da] hover:underline"
          >
            <Github size={14} />
            github.com/MuZiCul/XDownload
          </a>
        </div>
      </div>
    </div>
  );
}
