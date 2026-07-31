import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { startDownload, cancelDownload, checkYtdlp, checkFfmpeg } from "../../lib/bindings";
import type { DownloadConfig, VideoInfo } from "../../lib/types";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { Download, Ban, FileDown, Headphones, Monitor } from "lucide-react";

type Props = {
  config: DownloadConfig;
  videoInfo: VideoInfo | null;
  onConfigChange: (config: DownloadConfig) => void;
  onFormatQuick: (formatId: string) => void;
};

const QUICK_FORMATS = [
  { id: "best", label: "智能最佳", icon: FileDown, desc: "画质+音频" },
  { id: "bestvideo+bestaudio/best", label: "最高画质", icon: Monitor, desc: "视频+音频" },
  { id: "bestaudio", label: "仅音频", icon: Headphones, desc: "最佳音质" },
];

const DOWNLOAD_TOAST = "download";

export default function DownloadControls({
  config,
  videoInfo,
  onConfigChange,
  onFormatQuick,
}: Props) {
  const [downloading, setDownloading] = useState(false);

  const downloadMutation = useMutation({
    mutationFn: async (cfg: DownloadConfig) => {
      const ytStatus = await checkYtdlp();
      if (!ytStatus.available) {
        throw new Error("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp");
      }
      const ffStatus = await checkFfmpeg();
      if (!ffStatus.available) {
        throw new Error("ffmpeg 未安装，请先在设置页面的 Tools 中下载 ffmpeg");
      }

      const unlisten = await listen<any>("download-progress", (event) => {
        const p = event.payload;
        const eta = p.eta && p.eta !== "NA" ? ` 剩余 ${p.eta}` : "";
        toast.loading(`${p.percent}  ${p.speed || "?"}${eta}`, { id: DOWNLOAD_TOAST });
      });
      try {
        return await startDownload(cfg);
      } finally {
        (await unlisten)();
      }
    },
    onMutate: () => {
      setDownloading(true);
      toast.loading("正在下载...", { id: DOWNLOAD_TOAST });
    },
    onSuccess: (success) => {
      setDownloading(false);
      if (success) {
        toast.success("下载完成", { id: DOWNLOAD_TOAST });
      } else {
        toast.error("下载失败", { id: DOWNLOAD_TOAST });
      }
    },
    onError: (err: any) => {
      setDownloading(false);
      toast.error(`下载失败: ${err}`, { id: DOWNLOAD_TOAST });
    },
  });

  return (
    <div className="section-card">
      <div className="section-title">下载选项</div>

      <div className="grid grid-cols-3 gap-2 mb-4">
        {QUICK_FORMATS.map(({ id, label, icon: Icon, desc }) => (
          <label
            key={id}
            className={`flex flex-col items-center gap-1 px-2 py-2.5 rounded-xl border cursor-pointer transition-all duration-150 text-center ${
              config.format_id === id
                ? "border-blue-300 bg-blue-50/60 shadow-sm"
                : "border-zinc-200 hover:border-zinc-300 hover:bg-zinc-50"
            }`}
          >
            <Icon size={16} className="text-zinc-500 shrink-0" />
            <span className="text-[11px] font-medium">{label}</span>
            <span className="text-[10px] text-zinc-400">{desc}</span>
            <input
              type="radio"
              name="fmtQuick"
              value={id}
              checked={config.format_id === id}
              onChange={() => onFormatQuick(id)}
              className="sr-only"
            />
          </label>
        ))}
      </div>

      <div className="border-t border-zinc-100 pt-3 flex items-center gap-3 justify-end">
        <div className="flex items-center gap-1.5">
          <span className="text-[11px] text-zinc-400">重试</span>
          <input
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            value={config.retries}
            onChange={(e) => {
              const v = e.target.value.replace(/\D/g, "");
              onConfigChange({ ...config, retries: v ? parseInt(v, 10) : 5 });
            }}
            className="w-[36px]"
          />
        </div>

        {!downloading ? (
          <button
            className="btn btn-primary px-5 py-2 text-sm font-semibold flex items-center gap-2 shadow-sm"
            onClick={() => downloadMutation.mutate(config)}
            disabled={!videoInfo}
          >
            <Download size={15} />
            开始下载
          </button>
        ) : (
          <button
            className="btn btn-danger px-5 py-2 text-sm font-semibold flex items-center gap-2"
            onClick={() => cancelDownload()}
          >
            <Ban size={15} />
            取消下载
          </button>
        )}
      </div>
    </div>
  );
}
