import { useEffect, useState } from "react";
import {
  FolderOpen,
  History as HistoryIcon,
  RefreshCw,
  Trash2,
  Pause,
  Play,
  X,
  Loader2,
  ListPlus,
  Square,
} from "lucide-react";
import { openUrl, openPath } from "@tauri-apps/plugin-opener";
import CoverThumb from "../common/CoverThumb";
import ContextMenu, {
  copyLinkItem,
  openLinkItem,
  type ContextMenuItem,
} from "../common/ContextMenu";
import {
  listDownloadHistory,
  deleteDownloadHistory,
  clearDownloadHistory,
  openDownloadPath,
  loadSettings,
  checkVideoDownloaded,
} from "../../lib/bindings";
import type {
  DownloadHistoryItem,
  DownloadConfig,
  AppSettings,
} from "../../lib/types";
import {
  useDownloadStore,
  enqueueDownloadGlobal,
  pauseQueueTaskGlobal,
  resumeQueueTaskGlobal,
  removeQueueTaskGlobal,
  clearQueueTasksGlobal,
  pauseAllGlobal,
  resumeAllGlobal,
  cancelAllActiveGlobal,
  prepareQueue,
  refetchTaskInfoGlobal,
  type DownloadTask,
} from "../../lib/downloadStore";
import { friendlyErrorMessage } from "../../lib/errorMessages";
import BatchDownloadModal from "../download/BatchDownloadModal";
import DuplicateDownloadModal, {
  type DuplicateItem,
} from "../common/DuplicateDownloadModal";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";
import {
  formatDuration,
  formatNumber,
  formatDateTime,
  formatFileSize,
} from "../../lib/format";

type Props = {
  onRedownload: (item: DownloadHistoryItem) => void;
};

/** 任务面板：正在下载（实时队列）+ 下载完成（历史记录）。 */
export default function HistoryPage({ onRedownload }: Props) {
  const { t } = useI18n();
  const { queueTasks } = useDownloadStore();
  const [items, setItems] = useState<DownloadHistoryItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [batchOpen, setBatchOpen] = useState(false);
  // 批量入队时检测到「已下载」的链接，等待用户逐条处理（重新下载/取消）。
  const [duplicates, setDuplicates] = useState<DuplicateItem[]>([]);
  // 下载完成板块排序方式。
  const [sort, setSort] = useState<
    | "time"
    | "size"
    | "views"
    | "likes"
    | "duration"
    | "author"
    | "failed"
    | "missing"
  >("time");
  // 标题右键菜单（位置 + 菜单项）。
  const [ctx, setCtx] = useState<{
    x: number;
    y: number;
    items: ContextMenuItem[];
  } | null>(null);

  // 标题右键：复制下载链接 / 在浏览器打开。
  const openTitleMenu = (
    e: React.MouseEvent,
    url: string | null,
    hasLink: boolean
  ) => {
    e.preventDefault();
    e.stopPropagation();
    if (!url) return;
    const items: ContextMenuItem[] = [];
    if (hasLink)
      items.push(
        copyLinkItem(
          t("tasks.copyLink"),
          url,
          () => toast.success(t("tasks.copyLinkDone")),
          () => toast.error(t("tasks.copyLinkFail"))
        )
      );
    if (hasLink) items.push(openLinkItem(t("video.openInBrowser"), url));
    if (items.length === 0) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setCtx({ x: e.clientX, y: rect.bottom + 4, items });
  };

  const load = () => {
    setLoading(true);
    listDownloadHistory()
      .then(setItems)
      .catch(() => {})
      .finally(() => setLoading(false));
  };

  // 下载完成板块排序。
  const sortedItems = [...items].sort((a, b) => {
    switch (sort) {
      case "size":
        return (b.file_size ?? 0) - (a.file_size ?? 0);
      case "views":
        return b.view_count - a.view_count;
      case "likes":
        return b.like_count - a.like_count;
      case "duration":
        return b.duration - a.duration;
      case "author":
        return (a.uploader ?? "").localeCompare(b.uploader ?? "");
      case "failed":
        // 失败优先：失败排前，其余按时间倒序。
        const af = a.status === "failed" ? 1 : 0;
        const bf = b.status === "failed" ? 1 : 0;
        if (af !== bf) return bf - af;
        return b.downloaded_at - a.downloaded_at;
      case "missing":
        // 文件不存在（非失败）优先，其余按时间倒序。
        const am = !a.file_exists && a.status !== "failed" ? 1 : 0;
        const bm = !b.file_exists && b.status !== "failed" ? 1 : 0;
        if (am !== bm) return bm - am;
        return b.downloaded_at - a.downloaded_at;
      default:
        // 下载时间（默认，降序）
        return b.downloaded_at - a.downloaded_at;
    }
  });

  useEffect(() => {
    load();
  }, []);

  // 下载完成板块：任务终态后自动刷新历史（配合 download-finished 移除活跃任务）。
  useEffect(() => {
    // 每次进入页面 / 队列变化时刷新，保证与后端一致。
    load();
  }, [queueTasks.length]);

  const handleOpen = (id: string) => {
    openDownloadPath(id).catch((e: any) =>
      toast.error(t("video.openPathFail", { err: e }))
    );
  };

  // 用系统默认播放器播放已下载文件。
  const handlePlay = (path: string) => {
    toast.info(t("video.openingPlayer"));
    openPath(path).catch((e: any) =>
      toast.error(t("video.playFail", { err: e }))
    );
  };

  const handleRedownload = (item: DownloadHistoryItem) => {
    if (!item.url) {
      toast.warning(t("history.noUrl"));
      return;
    }
    onRedownload(item);
  };

  const handleOpenLink = (url: string | null) => {
    if (!url) return;
    openUrl(url).catch((e: any) =>
      toast.error(t("video.openUrlFail", { err: e }))
    );
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteDownloadHistory(id);
      setItems((prev) => prev.filter((i) => i.id !== id));
    } catch (e: any) {
      toast.error(t("history.deleteFail", { err: e }));
    }
  };

  const handleClear = async () => {
    try {
      await clearDownloadHistory();
      setItems([]);
      toast.success(t("history.cleared"));
    } catch (e: any) {
      toast.error(t("history.clearFail", { err: e }));
    }
  };

  /** 构建批量任务配置（元数据为空，由信息获取阶段补充）。 */
  const buildBatchConfig = (
    u: string,
    videoId: string | null,
    s: AppSettings | null
  ): DownloadConfig => ({
    url: u,
    video_id: videoId,
    title: null,
    thumbnail: null,
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
  });

  // 批量入队（两阶段）：已下载的链接先拦截到确认弹窗，其余全部入队但不立即
  // 下载（autoStart=false），随后触发信息获取流程 —— 所有任务先获取信息
  // （任务卡片显示「正在获取信息」），全部完成后自动从第一个开始下载。
  const handleBatchAdd = async (urls: string[]) => {
    const s = await loadSettings().catch(() => null);
    let added = 0;
    const dups: DuplicateItem[] = [];
    for (const u of urls) {
      const m = u.match(/\/status\/(\d+)/);
      const videoId = m ? m[1] : null;
      // 已下载检查：批量入队前拦截重复下载的链接，交用户逐条选择。
      if (videoId) {
        try {
          const st = await checkVideoDownloaded(videoId);
          if (st.downloaded) {
            dups.push({ url: u, video_id: videoId });
            continue;
          }
        } catch {
          // 检查失败不阻塞入队。
        }
      }
      try {
        await enqueueDownloadGlobal(buildBatchConfig(u, videoId, s), {
          autoStart: false,
        });
        added += 1;
      } catch (err: any) {
        toast.warning(friendlyErrorMessage(err));
      }
    }
    if (dups.length > 0) setDuplicates(dups);
    if (added > 0) {
      toast.success(t("queue.added"));
      prepareQueue(added); // 逐任务获取信息，每完成一个按并发尝试启动下载
    }
  };

  // 弹窗「重新下载」：立即加入队列并启动调度（autoStart=true），随后移除该项。
  const handleDupRedownload = async (item: DuplicateItem) => {
    const s = await loadSettings().catch(() => null);
    try {
      await enqueueDownloadGlobal(buildBatchConfig(item.url, item.video_id, s), {
        autoStart: true,
      });
      setDuplicates((prev) => prev.filter((x) => x.url !== item.url));
    } catch (err: any) {
      toast.warning(friendlyErrorMessage(err));
    }
  };

  // 弹窗「取消下载」：直接从列表移除，不入队。
  const handleDupCancel = (item: DuplicateItem) => {
    setDuplicates((prev) => prev.filter((x) => x.url !== item.url));
  };

  return (
    <div className="p-3 max-w-[900px] mx-auto">
      {/* ===== 正在下载 ===== */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <Loader2 size={15} className="text-blue-500" />
          <span className="text-[13px] font-semibold text-zinc-800">
            {t("tasks.downloading")}
          </span>
          {queueTasks.length > 0 && (
            <span className="text-[11px] text-zinc-400">
              {t("history.count", { count: queueTasks.length })}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          {queueTasks.length > 0 && (
            <>
              <button
                className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1"
                onClick={pauseAllGlobal}
                title={t("tasks.pauseAll")}
              >
                <Pause size={12} />
                {t("tasks.pauseAll")}
              </button>
              <button
                className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1"
                onClick={resumeAllGlobal}
                title={t("tasks.startAll")}
              >
                <Play size={12} />
                {t("tasks.startAll")}
              </button>
              <button
                className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1 text-red-600"
                onClick={cancelAllActiveGlobal}
                title={t("tasks.deleteAll")}
              >
                <Square size={12} />
                {t("tasks.deleteAll")}
              </button>
            </>
          )}
          <button
            className="btn px-3 py-1 text-xs font-semibold flex items-center gap-1"
            onClick={() => setBatchOpen(true)}
          >
            <ListPlus size={13} />
            {t("url.batch")}
          </button>
        </div>
      </div>

      <div className="section-card">
        {queueTasks.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-4">
            {t("tasks.emptyActive")}
          </p>
        ) : (
          <div className="divide-y divide-zinc-100">
            {queueTasks.map((task) => (
              <TaskCard
                key={task.id}
                task={task}
                onTitleMenu={(e, url, hasLink) =>
                  openTitleMenu(e, url, hasLink)
                }
              />
            ))}
          </div>
        )}
      </div>

      {/* ===== 下载完成 ===== */}
      <div className="flex items-center justify-between mb-2 mt-4">
        <div className="flex items-center gap-2">
          <HistoryIcon size={15} className="text-zinc-500" />
          <span className="text-[13px] font-semibold text-zinc-800">
            {t("tasks.completed")}
          </span>
          {!loading && items.length > 0 && (
            <span className="text-[11px] text-zinc-400">
              {t("history.count", { count: items.length })}
            </span>
          )}
        </div>
        {!loading && items.length > 0 && (
          <div className="flex items-center gap-1.5">
            <select
              value={sort}
              onChange={(e) => setSort(e.target.value as typeof sort)}
              className="text-xs py-1"
            >
              <option value="time">{t("history.sort.time")}</option>
              <option value="size">{t("history.sort.size")}</option>
              <option value="views">{t("history.sort.views")}</option>
              <option value="likes">{t("history.sort.likes")}</option>
              <option value="duration">{t("history.sort.duration")}</option>
              <option value="author">{t("history.sort.author")}</option>
              <option value="failed">{t("history.sort.failed")}</option>
              <option value="missing">{t("history.sort.missing")}</option>
            </select>
            <button
              className="btn px-3 py-1 text-xs font-semibold flex items-center gap-1 text-red-600"
              onClick={handleClear}
            >
              <Trash2 size={12} />
              {t("history.clearAll")}
            </button>
          </div>
        )}
      </div>

      <div className="section-card">
        {loading ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("common.loading")}
          </p>
        ) : items.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("history.empty")}
          </p>
        ) : (
          <div className="divide-y divide-zinc-100">
            {sortedItems.map((item) => (
              <div key={item.id} className="flex gap-3 py-3">
                <CoverThumb src={item.thumbnail} stretch />

                <div className="flex-1 min-w-0">
                  <p
                    onClick={() => handleOpenLink(item.url)}
                    onContextMenu={(e) => openTitleMenu(e, item.url, !!item.url)}
                    title={item.url ? t("video.openInBrowser") : undefined}
                    className={`text-[13px] font-semibold leading-snug truncate mb-2 ${
                      item.url
                        ? "cursor-pointer hover:text-blue-600 hover:underline"
                        : "text-zinc-900"
                    }`}
                  >
                    {item.title || item.id}
                  </p>

                  <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs mb-2">
                    <InfoRow label={t("video.author")} value={item.uploader || "—"} />
                    <InfoRow
                      label={t("video.duration")}
                      value={item.duration > 0 ? formatDuration(item.duration) : "—"}
                    />
                    <InfoRow
                      label={t("video.views")}
                      value={item.view_count > 0 ? formatNumber(item.view_count, t) : "—"}
                    />
                    <InfoRow
                      label={t("video.likes")}
                      value={item.like_count > 0 ? formatNumber(item.like_count, t) : "—"}
                    />
                  </div>

                  <div className="flex items-center gap-2 flex-wrap">
                    {item.status === "failed" ? (
                      <div
                        className="inline-flex items-center gap-1 text-[11px] font-medium text-red-700 bg-red-50 border border-red-200 rounded-md px-2 py-1"
                        title={
                          item.error
                            ? `${t("history.failed")}：${item.error}`
                            : t("history.failed")
                        }
                      >
                        {t("history.failed")}
                        {item.error && <span className="max-w-[180px] truncate">{item.error}</span>}
                        <span className="text-red-400">
                          {t("history.attempts", { count: item.attempts })}
                        </span>
                      </div>
                    ) : (
                      <div className="inline-flex items-center gap-1 text-[10px] font-medium text-zinc-600 bg-zinc-50 border border-zinc-200 rounded-md px-1.5 py-0.5">
                        {formatDateTime(item.downloaded_at)}
                        {item.file_size ? ` · ${formatFileSize(item.file_size)}` : ""}
                      </div>
                    )}
                    {item.status !== "failed" && !item.file_exists && (
                      <span className="inline-flex items-center gap-1 text-[10px] font-medium text-red-600 bg-red-50 border border-red-200 rounded-md px-1.5 py-0.5">
                        {t("history.fileMissing")}
                      </span>
                    )}
                    <div className="flex items-center gap-1 ml-auto">
                      {item.status !== "failed" && item.file_path && (
                        <button
                          className="p-1 rounded hover:bg-zinc-100 text-emerald-700 shrink-0 disabled:opacity-40 disabled:hover:bg-transparent"
                          onClick={() => handlePlay(item.file_path!)}
                          disabled={!item.file_exists}
                          title={t("video.play")}
                        >
                          <Play size={13} />
                        </button>
                      )}
                      {item.status !== "failed" && (
                        <button
                          className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-zinc-600 shrink-0 disabled:opacity-40 disabled:hover:bg-transparent"
                          onClick={() => handleOpen(item.id)}
                          disabled={!item.file_exists}
                          title={t("video.openPath")}
                        >
                          <FolderOpen size={13} />
                        </button>
                      )}
                      <button
                        className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-emerald-600 shrink-0"
                        onClick={() => handleRedownload(item)}
                        title={t("video.redownload")}
                      >
                        <RefreshCw size={13} />
                      </button>
                      <button
                        className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-red-600 shrink-0"
                        onClick={() => handleDelete(item.id)}
                        title={t("history.delete")}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {batchOpen && (
        <BatchDownloadModal
          onAdd={handleBatchAdd}
          onClose={() => setBatchOpen(false)}
        />
      )}

      {/* 已下载确认弹窗：所有链接处理完（列表清空）后自动关闭，无其他关闭途径。 */}
      {duplicates.length > 0 && (
        <DuplicateDownloadModal
          items={duplicates}
          onRedownload={handleDupRedownload}
          onCancel={handleDupCancel}
        />
      )}

      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          items={ctx.items}
          onClose={() => setCtx(null)}
        />
      )}
    </div>
  );
}

/** 正在下载的任务卡片 —— 布局与下载完成卡片一致，额外显示进度与控制。 */
function TaskCard({
  task,
  onTitleMenu,
}: {
  task: DownloadTask;
  onTitleMenu: (
    e: React.MouseEvent,
    url: string | null,
    hasLink: boolean
  ) => void;
}) {
  const { t } = useI18n();
  const info = task.info;
  const title = info?.title || task.title || task.url;
  const openLink = task.url || null;

  const statusBadge = () => {
    // 信息获取失败的任务：无论 paused/queued 都优先显示失败徽标。
    if (task.infoFailed) {
      return (
        <span className="text-[10px] text-red-600 bg-red-50 border border-red-200 rounded-md px-1.5 py-0.5 shrink-0">
          {t("tasks.infoFailed")}
        </span>
      );
    }
    switch (task.status) {
      case "downloading":
        return (
          <span className="text-[10px] text-blue-600 bg-blue-50 border border-blue-200 rounded-md px-1.5 py-0.5 shrink-0">
            {t("batch.downloading")}
          </span>
        );
      case "paused":
        return (
          <span className="text-[10px] text-amber-600 bg-amber-50 border border-amber-200 rounded-md px-1.5 py-0.5 shrink-0">
            {t("batch.paused")}
          </span>
        );
      default:
        // 排队中：
        //  - 正在被获取 → 正在获取信息
        //  - 未轮到获取 → 等待获取信息
        //  - 已获取 → 排队中（等待下载）
        if (!info?.title && !info?.thumbnail && !info?.uploader) {
          return task.infoFetching ? (
            <span className="text-[10px] text-sky-600 bg-sky-50 border border-sky-200 rounded-md px-1.5 py-0.5 shrink-0">
              {t("tasks.fetchingInfo")}
            </span>
          ) : (
            <span className="text-[10px] text-zinc-500 bg-zinc-50 border border-zinc-200 rounded-md px-1.5 py-0.5 shrink-0">
              {t("tasks.waitingInfo")}
            </span>
          );
        }
        return (
          <span className="text-[10px] text-zinc-600 bg-zinc-50 border border-zinc-200 rounded-md px-1.5 py-0.5 shrink-0">
            {t("tasks.waitingDownload")}
          </span>
        );
    }
  };

  return (
    <div className="flex gap-3 py-3">
      <CoverThumb src={info?.thumbnail ?? null} stretch />

      <div className="flex-1 min-w-0">
        <p
          onClick={() => openLink && openUrl(openLink).catch(() => {})}
          onContextMenu={(e) => onTitleMenu(e, openLink, !!openLink)}
          title={openLink ? t("video.openInBrowser") : undefined}
          className={`text-[13px] font-semibold leading-snug truncate mb-2 ${
            openLink ? "cursor-pointer hover:text-blue-600 hover:underline" : "text-zinc-900"
          }`}
        >
          {title}
        </p>

        <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs mb-2">
          <InfoRow label={t("video.author")} value={info?.uploader || "—"} />
          <InfoRow
            label={t("video.duration")}
            value={info && info.duration > 0 ? formatDuration(info.duration) : "—"}
          />
          <InfoRow
            label={t("video.views")}
            value={info && info.view_count > 0 ? formatNumber(info.view_count, t) : "—"}
          />
          <InfoRow
            label={t("video.likes")}
            value={info && info.like_count > 0 ? formatNumber(info.like_count, t) : "—"}
          />
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          {statusBadge()}
          {/* 进度条 + 阶段 + 速度：跟在徽标后面，不独占一行 */}
          {task.status === "downloading" && (
            <div className="flex items-center gap-2 min-w-0">
              <span className="text-[10px] text-zinc-500 shrink-0 whitespace-nowrap">
                {task.stage === "merge"
                  ? t("gbar.stageMerge")
                  : task.stage === "audio"
                    ? t("gbar.stageAudio")
                    : task.stage === "video"
                      ? t("gbar.stageVideo")
                      : t("gbar.progressLabel")}
              </span>
              {task.stage === "merge" ? (
                /* 合并/后期处理：仅不确定进度循环条 */
                <div className="relative w-40 h-1.5 bg-zinc-100 rounded-full overflow-hidden shrink-0">
                  <div
                    className="absolute inset-y-0 w-2/5 rounded-full animate-progress-run"
                    style={{
                      background: "linear-gradient(90deg, #3b82f6, #6366f1)",
                      boxShadow: "0 0 6px rgba(99, 102, 241, 0.4)",
                    }}
                  />
                </div>
              ) : (
                <>
                  <div className="relative w-40 h-1.5 bg-zinc-100 rounded-full overflow-hidden shrink-0">
                    {/* 渐变填充 */}
                    <div
                      className="absolute inset-y-0 left-0 rounded-full transition-all duration-200 ease-out"
                      style={{
                        width: `${Math.max(task.percent, 2)}%`,
                        background: "linear-gradient(90deg, #3b82f6, #6366f1)",
                        boxShadow: "0 0 6px rgba(99, 102, 241, 0.4)",
                      }}
                    />
                    {/* 扫过轨道的 shimmer 高光 */}
                    <div
                      className="absolute inset-y-0 w-10 blur-[2px] animate-progress-shimmer"
                      style={{ background: "linear-gradient(90deg, #3b82f6, #6366f1)" }}
                    />
                  </div>
                  <span className="text-[10px] tabular-nums text-zinc-600 w-9 text-right shrink-0">
                    {task.percent}%
                  </span>
                  <span className="text-[10px] tabular-nums text-zinc-400 w-16 text-right shrink-0">
                    {task.speed || "—"}
                  </span>
                </>
              )}
            </div>
          )}
          <div className="flex items-center gap-1 ml-auto">
            {task.infoFailed ? (
              <button
                className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-emerald-600"
                onClick={() => refetchTaskInfoGlobal(task.id)}
                title={t("tasks.retryInfo")}
              >
                <RefreshCw size={13} />
              </button>
            ) : task.status === "paused" ? (
              <button
                className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-emerald-600"
                onClick={() => resumeQueueTaskGlobal(task.id)}
                title={t("batch.resumeTask")}
              >
                <Play size={13} />
              </button>
            ) : task.status === "queued" || task.status === "downloading" ? (
              <button
                className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-amber-600"
                onClick={() => pauseQueueTaskGlobal(task.id)}
                title={t("batch.pauseTask")}
              >
                <Pause size={13} />
              </button>
            ) : null}
            <button
              className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-zinc-600"
              onClick={() => removeQueueTaskGlobal(task.id)}
              title={t("batch.remove")}
            >
              <X size={13} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="text-zinc-400 shrink-0">{label}</span>
      <span className="text-zinc-700 truncate">{value}</span>
    </div>
  );
}
