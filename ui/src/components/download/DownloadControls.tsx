import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import {
  startDownload,
  cancelDownload,
  checkYtdlp,
  checkFfmpeg,
  checkVideoDownloaded,
  loadSettings,
  openDownloadPath,
} from "../../lib/bindings";
import type { DownloadConfig, VideoInfo } from "../../lib/types";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { Download, Ban, FileDown, FolderOpen, Headphones, Monitor, RefreshCw } from "lucide-react";
import { friendlyErrorMessage } from "../../lib/errorMessages";

type Props = {
  config: DownloadConfig;
  videoInfo: VideoInfo | null;
  onConfigChange: (config: DownloadConfig) => void;
  onFormatQuick: (formatId: string) => void;
  onDownloadComplete?: () => void;
};

const QUICK_FORMATS = [
  { id: "best", label: "智能最佳", icon: FileDown, desc: "画质+音频" },
  { id: "bestvideo+bestaudio/best", label: "最高画质", icon: Monitor, desc: "视频+音频" },
  { id: "bestaudio", label: "仅音频", icon: Headphones, desc: "最佳音质" },
];

const DOWNLOAD_TOAST = "download";

type DlProgress = {
  percent: number;
  speed: string;
  eta: string;
  status: string; // downloading | merging | postprocess | finished
};

function statusText(status: string): string {
  switch (status) {
    case "merging":
      return "正在合并音视频流...";
    case "postprocess":
      return "正在后处理...";
    case "finished":
      return "下载完成";
    default:
      return "正在下载...";
  }
}

function formatDateTime(ts: number): string {
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export default function DownloadControls({
  config,
  videoInfo,
  onConfigChange,
  onFormatQuick,
  onDownloadComplete,
}: Props) {
  const [downloading, setDownloading] = useState(false);
  const [confirming, setConfirming] = useState(false);
  // The freshest config built right before download (used when the user
  // confirms a repeat download, so it also uses the latest settings).
  const [pendingCfg, setPendingCfg] = useState<DownloadConfig | null>(null);
  // Download progress shown as a visible progress bar in this card.
  const [dlProgress, setDlProgress] = useState<DlProgress | null>(null);

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
        const pct = parseFloat(String(p.percent ?? "").replace("%", "")) || 0;
        setDlProgress({
          percent: Math.min(Math.max(pct, 0), 100),
          speed: p.speed ?? "",
          eta: p.eta ?? "",
          status: p.status ?? "downloading",
        });
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
      setDlProgress({ percent: 0, speed: "", eta: "", status: "downloading" });
      toast.loading("正在下载...", { id: DOWNLOAD_TOAST });
    },
    onSuccess: (success) => {
      setDownloading(false);
      setDlProgress(null);
      if (success) {
        toast.success("下载完成", { id: DOWNLOAD_TOAST });
        onDownloadComplete?.();
      } else {
        toast.error("下载失败", { id: DOWNLOAD_TOAST });
      }
    },
    onError: (err: any) => {
      setDownloading(false);
      setDlProgress(null);
      toast.error(`下载失败: ${friendlyErrorMessage(err)}`, { id: DOWNLOAD_TOAST });
    },
  });

  // Before downloading, pull the latest settings from disk so the download
  // always uses fresh output_dir / cookies (avoids stale config from the
  // settings page). Uses a local variable — not React state — to avoid the
  // async setState race when mutating right after.
  const buildLatestConfig = async (): Promise<DownloadConfig> => {
    try {
      const s = await loadSettings();
      return {
        ...config,
        output_dir: s.download_dir ?? "downloads",
        cookies_from_browser: s.cookies_from_browser ?? null,
      };
    } catch {
      return config;
    }
  };

  // Before downloading, re-check on disk whether this video already exists.
  // This happens right before the request, so if the user deleted the file
  // after seeing the "已下载" hint, we download again without asking.
  const handleDownloadClick = async () => {
    const latest = await buildLatestConfig();
    setPendingCfg(latest);

    if (videoInfo?.id) {
      try {
        const status = await checkVideoDownloaded(videoInfo.id);
        if (status.downloaded) {
          setConfirming(true);
          return;
        }
      } catch {
        // Check failed — don't block the download.
      }
    }
    downloadMutation.mutate(latest);
  };

  const handleConfirmDownload = () => {
    setConfirming(false);
    downloadMutation.mutate(pendingCfg ?? config);
    setPendingCfg(null);
  };

  const alreadyDownloaded = !!videoInfo?.downloaded;

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

      {/* Download progress bar */}
      {downloading && dlProgress && (
        <div className="mb-4">
          <div className="flex items-center justify-between text-[11px] text-zinc-500 mb-1">
            <span className="font-medium text-zinc-700">
              {statusText(dlProgress.status)}
            </span>
            <span className="tabular-nums">
              {dlProgress.percent}%
              {dlProgress.speed ? `  ${dlProgress.speed}` : ""}
              {dlProgress.eta && dlProgress.eta !== "NA"
                ? `  剩余 ${dlProgress.eta}`
                : ""}
            </span>
          </div>
          <div className="w-full bg-gray-200/70 rounded-full h-2 overflow-hidden">
            <div
              className="h-full rounded-full transition-all duration-200 ease-out"
              style={{
                width: `${Math.max(dlProgress.percent, 2)}%`,
                background: "linear-gradient(90deg, #3b82f6, #6366f1)",
              }}
            />
          </div>
        </div>
      )}

      <div className="border-t border-zinc-100 pt-3 flex items-center gap-3 justify-end">
        {!downloading ? (
          <>
            {alreadyDownloaded && (
              <button
                className="btn px-5 py-2 text-sm font-semibold flex items-center gap-2 shadow-sm"
                onClick={() => {
                  if (!videoInfo?.id) return;
                  openDownloadPath(videoInfo.id).catch((e: any) =>
                    toast.error(`打开文件位置失败: ${e}`)
                  );
                }}
              >
                <FolderOpen size={15} />
                打开文件位置
              </button>
            )}
            <button
              className="btn btn-primary px-5 py-2 text-sm font-semibold flex items-center gap-2 shadow-sm"
              onClick={handleDownloadClick}
              disabled={!videoInfo}
            >
              {alreadyDownloaded ? <RefreshCw size={15} /> : <Download size={15} />}
              {alreadyDownloaded ? "重新下载" : "开始下载"}
            </button>
          </>
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

      {/* Repeat-download confirmation */}
      {confirming && (
        <div className="dialog-overlay" onClick={() => setConfirming(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-sm font-semibold text-zinc-900 mb-2">重复下载</h3>
            <p className="text-xs text-zinc-500 mb-4 leading-relaxed">
              该视频已下载
              {videoInfo?.downloaded_at
                ? `（${formatDateTime(videoInfo.downloaded_at)}）`
                : ""}
              ，是否重新下载？
            </p>
            <div className="flex gap-2 justify-end">
              <button className="btn" onClick={() => setConfirming(false)}>
                取消
              </button>
              <button
                className="btn btn-primary flex items-center gap-1"
                onClick={handleConfirmDownload}
              >
                <RefreshCw size={13} />
                重新下载
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
