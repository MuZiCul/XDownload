import { useState } from "react";
import { Github, RefreshCw } from "lucide-react";
import { checkUpdate } from "../../lib/bindings";
import { toast } from "sonner";

export default function AboutPage() {
  const [checking, setChecking] = useState(false);

  const handleCheckUpdate = async () => {
    if (checking) return;
    setChecking(true);

    try {
      const result = await checkUpdate();

      if (result.error) {
        toast.error(result.error);
      } else if (result.has_update) {
        toast(`发现新版本 v${result.latest_version}`, {
          description: `当前版本 v${result.current_version}`,
          action: {
            label: "前往下载",
            onClick: () => {
              const url =
                result.url ?? "https://github.com/MuZiCul/XDownload/releases";
              window.open(url, "_blank");
            },
          },
          duration: 8000,
        });
      } else {
        toast.success("已是最新版本");
      }
    } catch {
      toast.error("检测失败，请检查网络连接");
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
            <p className="text-sm text-gray-500">v2.5.0</p>
            <button
              className="inline-flex items-center gap-1 text-[11px] text-blue-500 hover:text-blue-600 hover:underline disabled:text-gray-300 disabled:no-underline transition-colors"
              onClick={handleCheckUpdate}
              disabled={checking}
            >
              <RefreshCw
                size={12}
                className={checking ? "animate-spin" : ""}
              />
              {checking ? "检测中..." : "检测更新"}
            </button>
          </div>

          <div className="h-5" />
          <p className="text-sm text-gray-600 leading-relaxed mb-1">
            基于 yt-dlp 的视频下载器
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
