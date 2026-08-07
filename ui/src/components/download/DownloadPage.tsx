import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import {
  fetchVideoInfo,
  loadSettings,
  checkYtdlp,
  checkFfmpeg,
  isFfmpegBundled,
  checkVideoDownloaded,
  openDownloadPath,
} from "../../lib/bindings";
import type { VideoInfo, DownloadConfig, DownloadHistoryItem } from "../../lib/types";
import { toast } from "sonner";
import UrlBar from "./UrlBar";
import VideoInfoCard from "./VideoInfoCard";
import FormatTable from "./FormatTable";
import {
  useDownloadStore,
  startDownloadGlobal,
  cancelDownloadGlobal,
} from "../../lib/downloadStore";
import { friendlyErrorMessage } from "../../lib/errorMessages";
import { useI18n } from "../../lib/i18n";

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
  const [confirming, setConfirming] = useState(false);
  // The freshest config built right before download (used when the user
  // confirms a repeat download, so it also uses the latest settings).
  const [pendingCfg, setPendingCfg] = useState<DownloadConfig | null>(null);
  // Global download state — survives tab switches / page unmounts.
  const { downloading, completed } = useDownloadStore();

  // Latest `downloading` value for event handlers registered once (which only
  // see the initial render's state).
  const downloadingRef = useRef(downloading);
  useEffect(() => {
    downloadingRef.current = downloading;
  }, [downloading]);

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
      if (downloadingRef.current) {
        toast.warning(t("history.busy"));
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
        startDownloadGlobal(cfg, { title: data.title ?? item.title });
      } catch (err: any) {
        toast.error(t("url.fetchFail", { err: friendlyErrorMessage(err) }), {
          id: "fetch-video",
        });
      }
    };
    window.addEventListener("history-redownload", handler);
    return () => window.removeEventListener("history-redownload", handler);
  }, [t]);

  // When a download completes, mark the video as downloaded so the info card
  // refreshes (badge + "重新下载") in real time.
  const prevCompleted = useRef(false);
  useEffect(() => {
    if (completed && !prevCompleted.current) {
      setVideoInfo((prev) =>
        prev
          ? {
              ...prev,
              downloaded: true,
              downloaded_at: Math.floor(Date.now() / 1000),
            }
          : prev
      );
    }
    prevCompleted.current = completed;
  }, [completed]);

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
    const ffStatus = await checkFfmpeg();
    if (!ffStatus.available) {
      throw new Error(t("tools.missing.ffmpeg"));
    }
    // ffmpeg 可用但不在内置 bin 目录（依赖系统 PATH）：告知用户最高画质
    // （音视频合并）可能无法下载，但不阻断下载。
    try {
      const bundled = await isFfmpegBundled();
      if (!bundled) {
        toast.warning(t("tools.ffmpegNotBundled"));
      }
    } catch {
      // 提示失败不影响下载
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
  const handleDownloadClick = async () => {
    if (downloading) return;
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

    try {
      await checkTools();
    } catch (err: any) {
      toast.error(err.message);
      return;
    }
    startDownloadGlobal(latest, { title: videoInfo?.title ?? null });
  };

  const handleConfirmDownload = async () => {
    setConfirming(false);
    try {
      await checkTools();
    } catch (err: any) {
      toast.error(err.message);
      setPendingCfg(null);
      return;
    }
    startDownloadGlobal(pendingCfg ?? config, {
      title: videoInfo?.title ?? null,
    });
    setPendingCfg(null);
  };

  const handleOpenPath = () => {
    if (!videoInfo?.id) return;
    openDownloadPath(videoInfo.id).catch((e: any) =>
      toast.error(t("video.openPathFail", { err: e }))
    );
  };

  return (
    <div className="p-3 max-w-[900px] mx-auto">
      <UrlBar onFetch={handleFetch} isLoading={fetchMutation.isPending} />

      <VideoInfoCard
        info={videoInfo}
        downloading={downloading}
        confirming={confirming}
        onDownload={handleDownloadClick}
        onConfirmDownload={handleConfirmDownload}
        onCancelConfirm={() => setConfirming(false)}
        onCancelDownload={cancelDownloadGlobal}
        onOpenPath={handleOpenPath}
      />

      <div className="mt-3">
        <FormatTable formats={videoInfo?.formats ?? []} />
      </div>
    </div>
  );
}
