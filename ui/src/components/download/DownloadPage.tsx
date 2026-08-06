import { useState, useEffect } from "react";
import { useMutation } from "@tanstack/react-query";
import { fetchVideoInfo, loadSettings, checkYtdlp } from "../../lib/bindings";
import type { VideoInfo, DownloadConfig } from "../../lib/types";
import { toast } from "sonner";
import UrlBar from "./UrlBar";
import VideoInfoCard from "./VideoInfoCard";
import FormatTable from "./FormatTable";
import DownloadControls from "./DownloadControls";

function defaultConfig(): DownloadConfig {
  return {
    url: "",
    video_id: null,
    format_id: "best",
    output_dir: "downloads",
    output_template: "%(title)s.%(ext)s",
    extract_audio: false,
    embed_subtitles: false,
    embed_thumbnail: false,
    write_thumbnail: false,
    proxy: null,
    retries: 5,
    socket_timeout: 30,
    cookies_file: null,
    cookies_from_browser: null,
    max_height: 0,
    download_archive: null,
  };
}

export default function DownloadPage() {
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [config, setConfig] = useState<DownloadConfig>(defaultConfig());

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
        throw new Error("yt-dlp 未安装，请先在设置页面的 Tools 中下载 yt-dlp");
      }
      return fetchVideoInfo(url);
    },
    onMutate: () => {
      toast.loading("正在获取视频信息...", { id: "fetch-video" });
    },
    onSuccess: (data) => {
      setVideoInfo(data);
      setConfig((c) => ({ ...c, url: data.url, video_id: data.id }));
      toast.success("获取成功", { id: "fetch-video" });
    },
    onError: (err: any) => {
      toast.error(`获取失败: ${err}`, { id: "fetch-video" });
    },
  });

  const handleFetch = (url: string) => {
    fetchMutation.mutate(url);
  };

  const handleFormatSelect = (formatId: string) => {
    setConfig((c) => ({ ...c, format_id: formatId }));
  };

  const handleFormatQuick = (formatId: string) => {
    setConfig((c) => ({ ...c, format_id: formatId }));
  };

  return (
    <div className="p-3 max-w-[900px] mx-auto">
      <UrlBar onFetch={handleFetch} isLoading={fetchMutation.isPending} />

      <div className="space-y-3" style={{ height: "calc(100vh - 145px)", overflow: "auto" }}>
        <VideoInfoCard info={videoInfo} />
        <FormatTable
          formats={videoInfo?.formats ?? []}
          selectedFormat={config.format_id}
          onSelect={handleFormatSelect}
        />
        <DownloadControls
          config={config}
          videoInfo={videoInfo}
          onConfigChange={setConfig}
          onFormatQuick={handleFormatQuick}
          onDownloadComplete={() => {
            // Mark the video as downloaded locally so the button becomes
            // "重新下载" and the info card shows the completion time.
            setVideoInfo((prev) =>
              prev
                ? {
                    ...prev,
                    downloaded: true,
                    downloaded_at: Math.floor(Date.now() / 1000),
                  }
                : prev
            );
          }}
        />
      </div>
    </div>
  );
}
