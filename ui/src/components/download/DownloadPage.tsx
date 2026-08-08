import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import {
  fetchVideoInfo,
  loadSettings,
  checkYtdlp,
  isFfmpegBundled,
  checkVideoDownloaded,
  openDownloadPath,
  startQueue,
} from "../../lib/bindings";
import type { VideoInfo, DownloadConfig, DownloadHistoryItem } from "../../lib/types";
import { toast } from "sonner";
import UrlBar from "./UrlBar";
import VideoInfoCard from "./VideoInfoCard";
import FormatTable from "./FormatTable";
import { useDownloadStore, enqueueDownloadGlobal } from "../../lib/downloadStore";
import { friendlyErrorMessage } from "../../lib/errorMessages";
import { useI18n } from "../../lib/i18n";
import { RefreshCw, ListVideo } from "lucide-react";

/** 下载前确认弹窗类型。 */
type ConfirmKind = "repeat" | "inQueue";

function defaultConfig(): DownloadConfig {
  return {
    url: "",
    video_id: null,
    title: null,
    thumbnail: null,
    // 固定智能最佳：合并最佳视频流+音频流，无分离流时降级到最佳单文件
    format_id: "bestvideo+bestaudio/best",
    output_dir: "downloads",
    output_template: "%(title)s.%(ext)s",
    extract_audio: false,
    embed_subtitles: false,
    embed_thumbnail: false,
    write_thumbnail: false,
    proxy: null,
    socket_timeout: 30,
    cookies_file: null,
    cookies_from_browser: null,
    max_height: 0,
    download_archive: null,
  };
}

export default function DownloadPage() {
  const { t } = useI18n();
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [config, setConfig] = useState<DownloadConfig>(defaultConfig());
  const [confirmKind, setConfirmKind] = useState<ConfirmKind | null>(null);
  // The freshest config built right before download (used when the user
  // confirms a download, so it also uses the latest settings).
  const [pendingCfg, setPendingCfg] = useState<DownloadConfig | null>(null);
  // Global download state — survives tab switches / page unmounts.
  const { queueTasks } = useDownloadStore();
  // URL 输入（受控）。
  const [url, setUrl] = useState("");

  // Re-download from the history page: `App` switches to this tab and fires
  // this event with a DownloadHistoryItem. We parse the URL with yt-dlp for
  // live data (format list, title, thumbnail, …), then fill the info card and
  // start the download automatically — history records are not re-used.
  useEffect(() => {
    const handler = async (e: Event) => {
      const item = (e as CustomEvent<DownloadHistoryItem>).detail;
      if (!item?.id || !item.url) {
        toast.warning(t("history.noUrl"));
        return;
      }
      toast.loading(t("prog.fetching"), { id: "fetch-video" });
      try {
        const ytStatus = await checkYtdlp();
        if (!ytStatus.available) {
          throw new Error(t("tools.missing.ytdlp"));
        }
        const data = await fetchVideoInfo(item.url);
        // Fill the info card with the freshly parsed data (incl. formats).
        setVideoInfo(data);
        setConfig((c) => ({
          ...c,
          url: data.url,
          video_id: data.id,
          title: data.title,
          thumbnail: data.thumbnail,
          uploader: data.uploader,
          duration: data.duration,
          view_count: data.view_count,
          like_count: data.like_count,
        }));
        toast.success(t("url.fetchOk"), { id: "fetch-video" });

        // Build a fresh config (latest settings) and start downloading.
        const s = await loadSettings().catch(() => null);
        const cfg: DownloadConfig = {
          url: data.url,
          video_id: data.id,
          title: data.title,
          thumbnail: data.thumbnail,
          uploader: data.uploader,
          duration: data.duration,
          view_count: data.view_count,
          like_count: data.like_count,
          format_id: "bestvideo+bestaudio/best",
          output_dir: s?.download_dir ?? "downloads",
          output_template: "%(title)s.%(ext)s",
          extract_audio: false,
          embed_subtitles: false,
          embed_thumbnail: false,
          write_thumbnail: false,
          proxy: null,
          socket_timeout: 30,
          cookies_file: null,
          cookies_from_browser: s?.cookies_from_browser ?? null,
          max_height: 0,
          download_archive: null,
        };
        await enqueueDownloadGlobal(cfg, { title: data.title ?? item.title });
      } catch (err: any) {
        toast.error(t("url.fetchFail", { err: friendlyErrorMessage(err) }), {
          id: "fetch-video",
        });
      }
    };
    window.addEventListener("history-redownload", handler);
    return () => window.removeEventListener("history-redownload", handler);
  }, [t]);

  // When active queue tasks all finish, re-check the current video's download
  // status so the info card refreshes (badge + "重新下载") in real time.
  const hadActiveTasks = useRef(false);
  useEffect(() => {
    const hasActive = queueTasks.some(
      (t) => t.status === "queued" || t.status === "downloading"
    );
    if (hadActiveTasks.current && !hasActive && videoInfo?.id) {
      checkVideoDownloaded(videoInfo.id)
        .then((s) => {
          if (s.downloaded) {
            setVideoInfo((prev) =>
              prev
                ? {
                    ...prev,
                    downloaded: true,
                    downloaded_at:
                      s.downloaded_at ?? Math.floor(Date.now() / 1000),
                  }
                : prev
            );
          }
        })
        .catch(() => {});
    }
    hadActiveTasks.current = hasActive;
  }, [queueTasks, videoInfo?.id]);

  // Load saved settings into download config on mount and when config is applied
  const reloadConfigFromSettings = () => {
    loadSettings()
      .then((s) => {
        setConfig((c) => ({
          ...c,
          output_dir: s.download_dir ?? "downloads",
          cookies_from_browser: s.cookies_from_browser ?? null,
        }));
      })
      .catch(() => {});
  };

  useEffect(() => {
    reloadConfigFromSettings();
    const handler = () => reloadConfigFromSettings();
    window.addEventListener("config-applied", handler);
    return () => window.removeEventListener("config-applied", handler);
  }, []);

  const fetchMutation = useMutation({
    mutationFn: async (url: string) => {
      const ytStatus = await checkYtdlp();
      if (!ytStatus.available) {
        throw new Error(t("tools.missing.ytdlp"));
      }
      return fetchVideoInfo(url);
    },
    onMutate: () => {
      toast.loading(t("prog.fetching"), { id: "fetch-video" });
    },
    onSuccess: (data) => {
      setVideoInfo(data);
      setConfig((c) => ({
        ...c,
        url: data.url,
        video_id: data.id,
        title: data.title,
        thumbnail: data.thumbnail,
        uploader: data.uploader,
        duration: data.duration,
        view_count: data.view_count,
        like_count: data.like_count,
      }));
      toast.success(t("url.fetchOk"), { id: "fetch-video" });
    },
    onError: (err: any) => {
      toast.error(t("url.fetchFail", { err: friendlyErrorMessage(err) }), {
        id: "fetch-video",
      });
    },
  });

  const handleFetch = (url: string) => {
    fetchMutation.mutate(url);
  };

  const checkTools = async () => {
    const ytStatus = await checkYtdlp();
    if (!ytStatus.available) {
      throw new Error(t("tools.missing.ytdlp"));
    }
    // 内置 ffmpeg 为下载硬性前置条件（音视频合并依赖它，PATH 中的 ffmpeg
    // 不被接受），缺失时直接阻断下载。
    let bundled = false;
    try {
      bundled = await isFfmpegBundled();
    } catch {
      // ignore
    }
    if (!bundled) {
      throw new Error(t("tools.ffmpegNotBundled"));
    }
  };

  // Before downloading, pull the latest settings from disk so the download
  // always uses fresh output_dir / cookies (avoids stale config from the
  // settings page).
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
  // Note: tool checks (yt-dlp --version / ffmpeg -version) are deferred to the
  // confirm / actual-download step so the repeat-download dialog responds
  // instantly instead of waiting for child-process startup.
  // 执行入队（工具检查 → enqueue → 启动调度）。
  const doEnqueue = async (cfg: DownloadConfig) => {
    try {
      await checkTools();
    } catch (err: any) {
      toast.error(err.message);
      return;
    }
    try {
      // 单任务入队不立即启动，交由队列统一调度（startQueue 按并发 pump）：
      // 并发有空位则开始下载，忙则排队等待；卡片信息由下载面板元数据填充。
      await enqueueDownloadGlobal(cfg, {
        title: videoInfo?.title ?? null,
        autoStart: false,
      });
      startQueue().catch(() => {});
      toast.success(t("queue.added"));
    } catch (err: any) {
      toast.warning(friendlyErrorMessage(err));
    }
  };

  const handleDownloadClick = async () => {
    const latest = await buildLatestConfig();
    setPendingCfg(latest);

    // 1. 已在下载队列中（排队/下载中/暂停）→ 弹窗抉择。
    if (queueTasks.some((task) => task.url === latest.url)) {
      setConfirmKind("inQueue");
      return;
    }

    // 2. 重下机制只检测文件是否存在（不看历史记录）：文件存在 → 重复确认；
    //    文件不存在 → 直接下载。
    if (videoInfo?.id) {
      try {
        const status = await checkVideoDownloaded(videoInfo.id);
        if (status.downloaded) {
          setConfirmKind("repeat");
          return;
        }
      } catch {
        // Check failed — don't block the download.
      }
    }

    // 3. 无冲突 → 直接入队。
    await doEnqueue(latest);
  };

  const handleConfirmDownload = async () => {
    setConfirmKind(null);
    await doEnqueue(pendingCfg ?? config);
    setPendingCfg(null);
  };

  const handleGoTasks = () => {
    setConfirmKind(null);
    setPendingCfg(null);
    window.dispatchEvent(new CustomEvent("switch-tab", { detail: "history" }));
  };

  const handleOpenPath = () => {
    if (!videoInfo?.id) return;
    openDownloadPath(videoInfo.id).catch((e: any) =>
      toast.error(t("video.openPathFail", { err: e }))
    );
  };

  // 当前视频已在队列中（排队/下载中/暂停）→ 按钮禁用并显示「下载中...」。
  const inQueue =
    !!videoInfo &&
    queueTasks.some((task) => !!task.url && task.url === videoInfo.url);

  return (
    <div className="p-3 max-w-[900px] mx-auto">
      <UrlBar
        url={url}
        onUrlChange={setUrl}
        onFetch={handleFetch}
        isLoading={fetchMutation.isPending}
      />

      <VideoInfoCard
        info={videoInfo}
        onDownload={handleDownloadClick}
        onOpenPath={handleOpenPath}
        inQueue={inQueue}
      />

      <div className="mt-3">
        <FormatTable formats={videoInfo?.formats ?? []} />
      </div>

      {/* 下载前确认弹窗：队列中 / 重复下载 */}
      {confirmKind && (
        <div
          className="dialog-overlay"
          onClick={() => {
            setConfirmKind(null);
            setPendingCfg(null);
          }}
        >
          <div
            className="dialog-content"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-sm font-semibold text-zinc-900 mb-2">
              {confirmKind === "inQueue"
                ? t("video.inQueueTitle")
                : t("video.repeatTitle")}
            </h3>
            <p className="text-xs text-zinc-500 mb-4 leading-relaxed">
              {confirmKind === "inQueue"
                ? t("video.inQueueBody")
                : t("video.repeatBody", { time: "" })}
            </p>
            <div className="flex gap-2 justify-end">
              <button
                className="btn"
                onClick={() => {
                  setConfirmKind(null);
                  setPendingCfg(null);
                }}
              >
                {t("common.cancel")}
              </button>
              {confirmKind === "inQueue" ? (
                <button
                  className="btn btn-primary flex items-center gap-1"
                  onClick={handleGoTasks}
                >
                  <ListVideo size={13} />
                  {t("video.goTasks")}
                </button>
              ) : (
                <button
                  className="btn btn-primary flex items-center gap-1"
                  onClick={handleConfirmDownload}
                >
                  <RefreshCw size={13} />
                  {t("video.redownload")}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
