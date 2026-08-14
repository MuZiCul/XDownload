import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
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
  ChevronUp,
  ChevronDown,
  ArrowUpToLine,
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
  deleteDownloadHistoryFile,
  clearDownloadHistory,
  openDownloadPath,
  loadSettings,
  checkVideoDownloaded,
} from "../../lib/bindings";
import type { DownloadHistoryItem } from "../../lib/types";
import { TaskSource, taskSourceKey } from "../../lib/types";
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
  reorderQueueTaskGlobal,
  buildBatchConfig,
  type DownloadTask,
} from "../../lib/downloadStore";
import { friendlyErrorMessage } from "../../lib/errorMessages";
import BatchDownloadModal from "../download/BatchDownloadModal";
import DuplicateDownloadModal, {
  type DuplicateItem,
} from "../common/DuplicateDownloadModal";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";
import { usePrivacyMode } from "../../lib/privacyMode";
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
  const privacy = usePrivacyMode();
  const { queueTasks } = useDownloadStore();
  // 排队 + 暂停任务可被重排（置顶/上移）。sortableIds 保持显示顺序。
  const sortableIds = queueTasks
    .filter((t) => t.status === "queued" || t.status === "paused")
    .map((t) => t.id);
  const firstSortableId = sortableIds[0];
  const [items, setItems] = useState<DownloadHistoryItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [batchOpen, setBatchOpen] = useState(false);
  // 断点续传开关（默认关）：开启时隐藏暂停/开始按钮。
  const [resumeSupport, setResumeSupport] = useState(false);
  // 历史记录删除确认：null = 未打开，否则为待删记录的 video_id 与文件路径。
  const [deleteTarget, setDeleteTarget] = useState<{
    videoId: string;
    filePath: string | null;
  } | null>(null);
  // 历史失败卡片展开的错误详情记录 id（null = 全部收起）。
  const [expandedErrId, setExpandedErrId] = useState<string | null>(null);
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
  // 内置时钟：仅每 60 秒刷新一次，避免渲染时反复调用 Date.now()，
  // 用于「最近下载」徽标分级。
  const [now, setNow] = useState(() => Date.now());
  // 下载完成搜索：边输入边过滤（与排序联动）。
  const [searchInput, setSearchInput] = useState("");
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
  // 先按搜索词过滤（标题/作者/链接/ID，不区分大小写），再应用排序，二者联动。
  const filteredItems = searchInput
    ? items.filter((item) => {
        const q = searchInput.toLowerCase();
        return [item.title, item.uploader, item.url, item.video_id].some(
          (v) => v != null && v.toLowerCase().includes(q)
        );
      })
    : items;

  const sortedItems = [...filteredItems].sort((a, b) => {
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

  // 「下载完成」列表虚拟滚动：只渲染可视区域的行，数据量大时 DOM 恒定。
  // 行高动态测量（measureElement + ResizeObserver），失败卡片展开错误详情时自动适配。
  const historyListRef = useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: sortedItems.length,
    getScrollElement: () => historyListRef.current,
    estimateSize: () => 140,
    overscan: 6,
  });

  useEffect(() => {
    load();
  }, []);

  // 断点续传开关：读取设置控制暂停/开始按钮显隐。
  useEffect(() => {
    loadSettings()
      .then((s) => setResumeSupport(s.resume_support ?? false))
      .catch(() => {});
  }, []);

  // 下载完成板块：任务终态后自动刷新历史（配合 download-finished 移除活跃任务）。
  useEffect(() => {
    // 每次进入页面 / 队列变化时刷新，保证与后端一致。
    load();
    // 下载完成（队列长度变化）时立即刷新时钟：让「最近下载」徽标以当前
    // 时刻立即计算档位，而不是等下一个 60 秒定时器 tick。
    setNow(Date.now());
  }, [queueTasks.length]);

  // 内置时钟：每 60 秒刷新一次，驱动「最近下载」徽标分级。
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 60_000);
    return () => clearInterval(timer);
  }, []);

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

  // 点删除按钮：弹确认窗，询问是否同时删除已下载的文件。
  const handleDelete = (item: DownloadHistoryItem) => {
    setDeleteTarget({ videoId: item.video_id, filePath: item.file_path ?? null });
  };

  // 「仅删除记录」：不碰磁盘文件。
  const handleDeleteRecord = async () => {
    if (!deleteTarget) return;
    const { videoId } = deleteTarget;
    setDeleteTarget(null);
    try {
      await deleteDownloadHistory(videoId);
      setItems((prev) => prev.filter((i) => i.video_id !== videoId));
    } catch (e: any) {
      toast.error(t("history.deleteFail", { err: e }));
    }
  };

  // 「删除记录和文件」：同时删除磁盘上的已下载文件。
  const handleDeleteRecordAndFile = async () => {
    if (!deleteTarget) return;
    const { videoId } = deleteTarget;
    setDeleteTarget(null);
    try {
      await deleteDownloadHistoryFile(videoId, true);
      setItems((prev) => prev.filter((i) => i.video_id !== videoId));
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
          source: TaskSource.Batch,
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
        source: TaskSource.Batch,
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
    <div className="p-3 max-w-[900px] mx-auto h-full flex flex-col gap-2">
      {/* ===== 正在下载（30% 高度，内部滚动） ===== */}
      <section className="h-[30%] flex flex-col min-h-0">
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
              {resumeSupport ? (
                <span
                  className="inline-flex items-center gap-1 px-2.5 py-1 text-xs font-semibold border border-emerald-200 bg-emerald-50 text-emerald-700 rounded-xl"
                  title={t("tasks.resumeActive")}
                >
                  {t("tasks.resumeActive")}
                </span>
              ) : (
                <>
                  {queueTasks.some((t) => t.status === "downloading") ? (
                    <button
                      className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1"
                      onClick={pauseAllGlobal}
                      title={t("tasks.pauseAll")}
                    >
                      <Pause size={12} />
                      {t("tasks.pauseAll")}
                    </button>
                  ) : (
                    <button
                      className="btn px-2.5 py-1 text-xs font-semibold flex items-center gap-1"
                      onClick={resumeAllGlobal}
                      title={t("tasks.startAll")}
                    >
                      <Play size={12} />
                      {t("tasks.startAll")}
                    </button>
                  )}
                </>
              )}
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

      <div className="flex-1 overflow-y-auto min-h-0">
        {queueTasks.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-4">
            {t("tasks.emptyActive")}
          </p>
        ) : (
          <div className="space-y-2">
            {queueTasks.map((task) => (
              <TaskCard
                key={task.id}
                task={task}
                hidePause={resumeSupport}
                isFirst={firstSortableId === task.id}
                onMoveUp={() => {
                  // 按任务自身所在列表（暂停区/排队区各自内部）计算索引，
                  // 与后端 reorder_queue 的单列表语义一致，避免混合索引错位。
                  const sameListIds = queueTasks
                    .filter(
                      (t) => t.status === task.status && !t.infoFailed
                    )
                    .map((t) => t.id);
                  const idx = sameListIds.indexOf(task.id);
                  if (idx > 0) reorderQueueTaskGlobal(task.id, idx - 1);
                }}
                onMoveTop={() => reorderQueueTaskGlobal(task.id, 0)}
                onTitleMenu={(e, url, hasLink) =>
                  openTitleMenu(e, url, hasLink)
                }
              />
            ))}
          </div>
        )}
      </div>
      </section>

      {/* ===== 下载完成（剩余 70%，内部滚动 + 虚拟化） ===== */}
      <section className="flex-1 flex flex-col min-h-0">
      <div className="flex items-center justify-between mb-2">
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
            <input
              type="text"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              placeholder={t("history.searchPlaceholder")}
              className="w-44 text-xs py-1"
            />
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

      <div ref={historyListRef} className="flex-1 overflow-y-auto min-h-0">
        {loading ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("common.loading")}
          </p>
        ) : items.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("history.empty")}
          </p>
        ) : filteredItems.length === 0 ? (
          <p className="text-xs text-gray-400 text-center py-6">
            {t("history.noMatch")}
          </p>
        ) : (
          <div className="relative w-full" style={{ height: rowVirtualizer.getTotalSize() }}>
            {rowVirtualizer.getVirtualItems().map((vi) => {
              const item = sortedItems[vi.index];
              return (
                <div
                  key={item.video_id}
                  data-index={vi.index}
                  ref={rowVirtualizer.measureElement}
                  className="absolute top-0 left-0 w-full pb-2"
                  style={{ transform: `translateY(${vi.start}px)` }}
                >
                  <div className="flex gap-3 py-3 px-3 rounded-xl border border-zinc-200/80 bg-white
                             shadow-[1px_2px_6px_rgba(0,0,0,0.12)] transition-shadow
                             hover:shadow-[2px_3px_10px_rgba(59,130,246,0.35)]"
                  >
                <CoverThumb src={item.thumbnail} stretch blurred={privacy} />

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
                    {privacy ? "***" : item.title || item.video_id}
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
                    <SourceBadge source={item.source} />
                    {item.status === "failed" ? (
                      <>
                        <button
                          className="inline-flex items-center gap-1 text-[11px] font-medium text-red-700 bg-red-50 border border-red-200 rounded-md px-2 py-1 hover:bg-red-100"
                          onClick={() =>
                            setExpandedErrId((prev) =>
                              prev === item.video_id ? null : item.video_id
                            )
                          }
                          title={
                            item.error
                              ? `${t("history.failed")}：${item.error}`
                              : t("history.failed")
                          }
                        >
                          {t("history.failed")}
                          {item.error && (
                            <span className="max-w-[180px] truncate">
                              {item.error}
                            </span>
                          )}
                          <span className="text-red-400">
                            {t("history.attempts", { count: item.attempts })}
                          </span>
                          {item.error &&
                            (expandedErrId === item.video_id ? (
                              <ChevronUp size={11} />
                            ) : (
                              <ChevronDown size={11} />
                            ))}
                        </button>
                        {expandedErrId === item.video_id && item.error && (
                          <pre className="w-full text-[11px] text-red-600 bg-red-50 border border-red-100 rounded-md px-2 py-1.5 whitespace-pre-wrap break-all select-text max-h-40 overflow-auto">
                            {item.error}
                          </pre>
                        )}
                      </>
                    ) : (
                      <div className="inline-flex items-center gap-1 text-[10px] font-medium text-zinc-600 bg-zinc-50 border border-zinc-200 rounded-md px-1.5 py-0.5">
                        {formatDateTime(item.downloaded_at)}
                        {item.file_size ? ` · ${formatFileSize(item.file_size)}` : ""}
                      </div>
                    )}
                    {(() => {
                      const bucket = getRecentBucket(item.downloaded_at, now);
                      return bucket ? (
                        <span
                          className={`inline-flex items-center text-[10px] font-medium border rounded-md px-1.5 py-0.5 ${bucket.cls}`}
                        >
                          {t(bucket.key)}
                        </span>
                      ) : null;
                    })()}
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
                          onClick={() => handleOpen(item.video_id)}
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
                        onClick={() => handleDelete(item)}
                        title={t("history.delete")}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
      </section>

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

      {/* 删除历史记录确认弹窗：询问是否同时删除已下载的文件。 */}
      {deleteTarget && (
        <div
          className="dialog-overlay"
          onClick={() => setDeleteTarget(null)}
        >
          <div
            className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-[460px] max-w-[92vw] bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/50 p-5"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between mb-1">
              <span className="text-sm font-semibold text-zinc-900">
                {t("history.deleteTitle")}
              </span>
              <button
                className="p-1 rounded hover:bg-zinc-100 text-zinc-400"
                onClick={() => setDeleteTarget(null)}
              >
                <X size={14} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-2 leading-relaxed">
              {t("history.deleteBody")}
            </p>
            {deleteTarget.filePath && (
              <p
                className="text-[11px] text-zinc-400 mb-3 px-2 py-1.5 bg-zinc-100/70 rounded truncate"
                title={deleteTarget.filePath}
              >
                {deleteTarget.filePath}
              </p>
            )}
            <div className="flex items-center justify-end gap-2">
              <button
                className="btn px-2.5 py-1.5 text-xs font-semibold"
                onClick={() => setDeleteTarget(null)}
              >
                {t("history.deleteCancel")}
              </button>
              <button
                className="btn px-2.5 py-1.5 text-xs font-semibold"
                onClick={handleDeleteRecord}
              >
                {t("history.deleteRecordOnly")}
              </button>
              <button
                className="btn px-2.5 py-1.5 text-xs font-semibold text-red-600"
                onClick={handleDeleteRecordAndFile}
              >
                {t("history.deleteRecordAndFile")}
              </button>
            </div>
          </div>
        </div>
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

/** 「最近下载」分级档位：分钟数上限 → i18n 键 + 徽标颜色类。
 *  每条只显示匹配的最小档；超过 10 年（3650 天）不显示徽标。
 *  前五档按重要程度着色（越新越醒目），其余保持默认蓝。 */
const RECENT_BUCKETS: Array<{ maxMinutes: number; key: string; cls: string }> = [
  { maxMinutes: 5, key: "history.recent.5m", cls: "text-rose-600 bg-rose-50 border-rose-200" },
  { maxMinutes: 10, key: "history.recent.10m", cls: "text-orange-600 bg-orange-50 border-orange-200" },
  { maxMinutes: 15, key: "history.recent.15m", cls: "text-amber-600 bg-amber-50 border-amber-200" },
  { maxMinutes: 25, key: "history.recent.25m", cls: "text-yellow-600 bg-yellow-50 border-yellow-200" },
  { maxMinutes: 45, key: "history.recent.45m", cls: "text-green-600 bg-green-50 border-green-200" },
  { maxMinutes: 60, key: "history.recent.1h", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 180, key: "history.recent.3h", cls: "text-teal-600 bg-teal-50 border-teal-200" },
  { maxMinutes: 300, key: "history.recent.5h", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 720, key: "history.recent.12h", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 1440, key: "history.recent.24h", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 2880, key: "history.recent.yesterday", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 4320, key: "history.recent.dayBefore", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 10080, key: "history.recent.1w", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 30240, key: "history.recent.3w", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 43200, key: "history.recent.1mo", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 129600, key: "history.recent.3mo", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 262080, key: "history.recent.6mo", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 525600, key: "history.recent.1y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 1051200, key: "history.recent.2y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 1576800, key: "history.recent.3y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 2102400, key: "history.recent.4y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 2628000, key: "history.recent.5y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 3153600, key: "history.recent.6y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 3679200, key: "history.recent.7y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 4204800, key: "history.recent.8y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 4730400, key: "history.recent.9y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
  { maxMinutes: 5256000, key: "history.recent.10y", cls: "text-blue-600 bg-blue-50 border-blue-200" },
];

/** 根据下载时间与当前时钟返回最近下载徽标档位（key + 颜色类），超过 10 年返回 null。 */
function getRecentBucket(
  downloadedAt: number,
  nowMs: number
): { key: string; cls: string } | null {
  // downloaded_at 是 Unix 秒，Date.now() 是毫秒，需统一单位再计算。
  // 负数（下载刚完成、now 尚未刷新）clamp 到 0，立即命中「最近5分钟」，
  // 避免刚下载完的记录因被误判为「未来时间」而隐藏徽标。
  const elapsedMin = Math.max(0, (nowMs - downloadedAt * 1000) / 60_000);
  if (elapsedMin > 5256000) return null;
  const bucket = RECENT_BUCKETS.find((b) => elapsedMin <= b.maxMinutes);
  return bucket ? { key: bucket.key, cls: bucket.cls } : null;
}

/** 正在下载的任务卡片 —— 布局与下载完成卡片一致，额外显示进度与控制。 */
function TaskCard({
  task,
  isFirst,
  hidePause,
  onMoveUp,
  onMoveTop,
  onTitleMenu,
}: {
  task: DownloadTask;
  /** 该任务是否为可排序列表（queued+paused）中的第一个。 */
  isFirst?: boolean;
  /** 断点续传开启时隐藏暂停/继续按钮。 */
  hidePause?: boolean;
  onMoveUp?: () => void;
  onMoveTop?: () => void;
  onTitleMenu: (
    e: React.MouseEvent,
    url: string | null,
    hasLink: boolean
  ) => void;
}) {
  const { t } = useI18n();
  const privacy = usePrivacyMode();
  const info = task.info;
  const title = privacy ? "***" : info?.title || task.title || task.url;
  const openLink = task.url || null;
  // 信息获取失败的错误详情展开/收起。
  const [errOpen, setErrOpen] = useState(false);

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
    <div
      className="flex gap-3 py-3 px-3 rounded-xl border border-zinc-200/80 bg-white
                 shadow-[1px_2px_6px_rgba(0,0,0,0.12)] transition-shadow
                 hover:shadow-[2px_3px_10px_rgba(59,130,246,0.35)]"
    >
      <CoverThumb src={info?.thumbnail ?? null} stretch blurred={privacy} />

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
          <SourceBadge source={task.source} />
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
            {(task.status === "queued" || task.status === "paused") &&
              !task.infoFailed &&
              !isFirst && (
              <>
                <button
                  className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-blue-600"
                  onClick={onMoveUp}
                  title={t("tasks.moveUp")}
                >
                  <ChevronUp size={13} />
                </button>
                <button
                  className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-blue-600"
                  onClick={onMoveTop}
                  title={t("tasks.moveTop")}
                >
                  <ArrowUpToLine size={13} />
                </button>
              </>
            )}
            {!hidePause &&
              (task.infoFailed ? (
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
              ) : null)}
            <button
              className="p-1 rounded hover:bg-zinc-100 text-zinc-400 hover:text-zinc-600"
              onClick={() => removeQueueTaskGlobal(task.id)}
              title={t("batch.remove")}
            >
              <X size={13} />
            </button>
          </div>
          {task.infoFailed && task.error && (
            <div className="mt-1">
              <button
                className="flex items-center gap-1 text-[11px] text-red-500 hover:text-red-700 max-w-full"
                onClick={() => setErrOpen((v) => !v)}
                title={errOpen ? t("tasks.errHide") : t("tasks.errShow")}
              >
                {errOpen ? <ChevronUp size={11} /> : <ChevronDown size={11} />}
                <span className="truncate">
                  {errOpen ? t("tasks.errHide") : t("tasks.errShow")}
                </span>
              </button>
              {errOpen && (
                <pre className="mt-1 text-[11px] text-red-600 bg-red-50 border border-red-100 rounded-md px-2 py-1.5 whitespace-pre-wrap break-all select-text max-h-40 overflow-auto">
                  {task.error}
                </pre>
              )}
            </div>
          )}
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

/** 任务来源徽标：书签 / 批量 / 单链。无来源或未知来源时返回 null。 */
function SourceBadge({ source }: { source: string | undefined | null }) {
  const { t } = useI18n();
  const key = taskSourceKey(source);
  if (!key) return null;
  // 按来源配色：书签=紫、批量=青、单链=灰。
  const cls =
    source === TaskSource.Bookmark
      ? "text-purple-600 bg-purple-50 border-purple-200"
      : source === TaskSource.Batch
        ? "text-cyan-700 bg-cyan-50 border-cyan-200"
        : "text-zinc-500 bg-zinc-50 border-zinc-200";
  return (
    <span
      className={`inline-flex items-center text-[10px] font-medium border rounded-md px-1.5 py-0.5 shrink-0 ${cls}`}
    >
      {t(key)}
    </span>
  );
}
