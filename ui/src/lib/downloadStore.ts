import { useSyncExternalStore, createElement } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { toast } from "sonner";
import {
  enqueueDownload,
  cancelQueueTask,
  cancelAllTasks,
  clearDownloadQueue,
  reorderQueueTask,
  startQueue,
  pauseQueue,
  resumeQueue,
  pauseQueueTask,
  resumeQueueTask,
  pauseAllTasks,
  resumeAllTasks,
  queueStatus,
  updateTaskInfo,
  fetchVideoInfo,
  openFilePath,
} from "./bindings";
import type { AppSettings, DownloadConfig } from "./types";
import { friendlyErrorMessage } from "./errorMessages";
import { t as i18nT } from "./i18n";

export interface DlProgress {
  percent: number;
  speed: string;
  eta: string;
  status: string; // downloading | merging | postprocess | finished
  stage: string; // video | audio | merge | ""
}

/** Video metadata shown on task cards (filled at enqueue when available,
 *  otherwise fetched once the task starts). */
export interface TaskInfo {
  thumbnail: string | null;
  uploader: string | null;
  duration: number;
  view_count: number;
  like_count: number;
  title: string | null;
}

/** A task in the download queue. */
export interface DownloadTask {
  id: string;
  url: string;
  title: string | null;
  status:
    | "queued"
    | "downloading"
    | "moving"
    | "paused"
    | "completed"
    | "failed"
    | "cancelled";
  percent: number;
  speed: string;
  stage: string;
  error?: string;
  info?: TaskInfo;
  /** 信息获取失败（不再重试、不阻塞队列启动）。 */
  infoFailed?: boolean;
  /** 当前正在获取信息（用于「正在获取信息」徽标）。 */
  infoFetching?: boolean;
  /** 任务来源：single | batch | bookmark。 */
  source?: string;
}

export interface DownloadState {
  /** Active queue snapshot (queued / downloading / paused). */
  queueTasks: DownloadTask[];
}

const initialState: DownloadState = {
  queueTasks: [],
};

// Module-level state so it survives component unmounts (tab switches).
let state: DownloadState = initialState;
const listeners = new Set<() => void>();

function setState(partial: Partial<DownloadState>) {
  state = { ...state, ...partial };
  listeners.forEach((l) => l());
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function getSnapshot(): DownloadState {
  return state;
}

/** Merge a partial update into a queue task by id (no-op when absent). */
function patchTask(id: string, patch: Partial<DownloadTask>) {
  setState({
    queueTasks: state.queueTasks.map((t) =>
      t.id === id ? { ...t, ...patch } : t
    ),
  });
}

/** Whether a task already has usable video metadata (not an all-null stub). */
function hasValidInfo(info: TaskInfo | undefined): boolean {
  return !!info && (!!info.title || !!info.thumbnail || !!info.uploader);
}

/** Fetch video info for a task and fill its card metadata (best-effort). */
function fetchAndFillInfo(id: string, url: string) {
  fetchAndFillInfoAsync(id, url).catch(() => {});
}

/** Awaitable version used by the two-phase info fetch flow. */
async function fetchAndFillInfoAsync(id: string, url: string): Promise<void> {
  try {
    const data = await fetchVideoInfo(url);
    const info: TaskInfo = {
      thumbnail: data.thumbnail,
      uploader: data.uploader,
      duration: data.duration,
      view_count: data.view_count,
      like_count: data.like_count,
      title: data.title,
    };
    patchTask(id, { info, infoFailed: undefined });
    // 回写后端持久化，保证「保存进度并退出 → 重启」后卡片信息不丢失。
    // 失败仅影响下次重启的信息恢复，不影响本次展示与下载。
    updateTaskInfo(id, info).catch(() => {});
  } catch (e: any) {
    // 获取失败：标记 infoFailed 并记录具体原因。仅当任务还在排队（queued）
    // 时才暂停并跳过下载——这是「两阶段」流程的正常兜底。若任务已开始下载
    // （download-started 兜底再 fetch 失败），不中断正在进行的下载，
    // 避免网络波动时下载被意外 kill。
    patchTask(id, { infoFailed: true, error: friendlyErrorMessage(e) });
    const t = state.queueTasks.find((x) => x.id === id);
    if (t && t.status === "queued") {
      pauseQueueTask(id).catch(() => {});
    }
  }
}

/** 两阶段信息获取流程状态。 */
const infoFetchState = {
  running: false,
  paused: false,
  /** 期望入队的任务数（等待 download-queued 事件全部到达后再开始）。 */
  expected: 0,
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** 等待某任务真正开始下载（有进度），最多 2 秒；任务被暂停/失败则立即返回。 */
async function waitForTaskProgress(id: string) {
  for (let i = 0; i < 20; i++) {
    const t = state.queueTasks.find((x) => x.id === id);
    if (!t) return;
    if (t.status === "downloading" && t.percent > 0) return;
    if (t.status === "paused" || t.infoFailed) return;
    await sleep(100);
  }
}

/**
 * 逐任务获取信息（顺序）：获取第 N 个任务的信息 → startQueue 尝试启动
 * （按并发 pump）→ 等待该任务真正开始下载且有进度（或 2 秒超时）→ 再获取
 * 下一个。可被 pauseInfoFetch 中断、resumeInfoFetch 继续。
 */
async function runInfoFetch() {
  if (infoFetchState.running || infoFetchState.paused) return;
  infoFetchState.running = true;
  try {
    // 等待所有入队事件到达，避免 queueTasks 不全导致漏处理。
    if (infoFetchState.expected > 0) {
      for (let i = 0; i < 50; i++) {
        if (state.queueTasks.length >= infoFetchState.expected) break;
        await sleep(100);
      }
      infoFetchState.expected = 0;
    }

    while (!infoFetchState.paused) {
      // 只处理排队中的任务；已暂停（paused）的任务不获取信息，避免误标
      // 「信息获取失败」。
      const task = state.queueTasks.find(
        (t) =>
          !hasValidInfo(t.info) &&
          !t.infoFailed &&
          t.status === "queued"
      );
      if (!task) break;
      // 标记"正在获取信息"（供徽标区分）。
      patchTask(task.id, { infoFetching: true });
      await fetchAndFillInfoAsync(task.id, task.url);
      patchTask(task.id, { infoFetching: undefined });
      // 尝试启动：后端按并发数 pump，并发有空位则立即下载。
      startQueue();
      // 等待该任务开始下载（有进度）或 2 秒超时，再获取下一个。
      await waitForTaskProgress(task.id);
    }
    // 兜底：最后再尝试一次（并发空位由后端 pump 决定）。
    startQueue();
  } finally {
    infoFetchState.running = false;
  }
}

function pauseInfoFetch() {
  infoFetchState.paused = true;
}

function resumeInfoFetch() {
  infoFetchState.paused = false;
  runInfoFetch();
}

/** Subscribe a component to the global download state. */
export function useDownloadStore(): DownloadState {
  return useSyncExternalStore(subscribe, getSnapshot);
}

// Re-exported from buildConfig.ts (pure, unit-tested). Kept here so existing
// imports in HistoryPage / BookmarksSetting keep working.
export { buildBatchConfig } from "./buildConfig";

/**
 * Send a system notification (shown even when the window is minimized to the
 * tray). Silently no-ops when permission is denied or the plugin is missing.
 */
async function notify(title: string, body: string) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (granted) {
      sendNotification({ title, body });
    }
  } catch {
    // Notifications are best-effort; never break the download flow.
  }
}

let initialized = false;

/**
 * Register global download event listeners (once) and restore the queue
 * snapshot. Must be called once, e.g. from App's useEffect.
 */
export function initDownloadStore() {
  if (initialized) return;
  initialized = true;

  listen<any>("download-progress", (event) => {
    const p = event.payload;
    const id = String(p?.task_id ?? "");
    if (!id) return;
    const pct = parseFloat(String(p.percent ?? "").replace("%", "")) || 0;
    patchTask(id, {
      status: p.status === "moving" ? "moving" : "downloading",
      percent: Math.min(Math.max(pct, 0), 100),
      speed: p.speed ?? "",
      stage: p.stage ?? "",
      error: undefined,
    });
  });

  // ---- Queue events ----

  listen<any>("download-queued", (event) => {
    const p = event.payload ?? {};
    const id = String(p.task_id ?? "");
    if (!id) return;
    const existing = state.queueTasks.find((t) => t.id === id);
    if (existing) {
      // 恢复队列时事件可能与 queueStatus 回填重复到达：已存在的任务只合并
      // 缺失的卡片信息，不覆盖当前进度/状态（避免重复卡片，也避免把
      // downloading 降级为 queued）。
      const eventInfo = p.info as TaskInfo | undefined;
      if (eventInfo && !hasValidInfo(existing.info)) {
        patchTask(id, { info: eventInfo });
      }
      return;
    }
    setState({
      queueTasks: [
        ...state.queueTasks,
        {
          id,
          url: p.url ?? "",
          title: p.title ?? null,
          // 恢复队列时后端可能带 status（paused 保持暂停显示）。
          status: p.status === "paused" ? "paused" : "queued",
          percent: 0,
          speed: "",
          stage: "",
          info: p.info as TaskInfo | undefined,
          source: typeof p.source === "string" ? p.source : undefined,
        },
      ],
    });
  });

  // 深链（浏览器扩展）批量入队成功：合并提示并跳转到任务页。
  // 已下载过的链接（duplicates）由后端单独收集，转发全局事件交 App 弹窗
  // 让用户逐条选择「重新下载 / 取消」。
  listen<any>("deep-link-queued", (event) => {
    const count = Number(event.payload?.count ?? 1);
    const dups = Array.isArray(event.payload?.duplicates)
      ? (event.payload.duplicates as { url: string; video_id: string | null }[])
      : [];
    if (dups.length > 0) {
      window.dispatchEvent(
        new CustomEvent("deep-link-duplicates", { detail: dups })
      );
    }
    if (count > 0) {
      const msg =
        count > 1
          ? i18nT("gbar.deepLinkQueuedBatch", { n: count })
          : i18nT("gbar.deepLinkQueued");
      toast.success(
        createElement(
          "div",
          { className: "flex items-center gap-1.5 min-w-0 w-full" },
          createElement(
            "span",
            { className: "truncate min-w-0 text-zinc-700" },
            msg
          )
        ),
        { id: "deep-link-global" }
      );
    }
    window.dispatchEvent(new CustomEvent("switch-tab", { detail: "history" }));
  });

  listen<any>("download-started", (event) => {
    const id = String(event.payload?.task_id ?? "");
    if (!id) return;
    const existing = state.queueTasks.find((t) => t.id === id);
    patchTask(id, { status: "downloading", percent: 0 });
    // 兜底：任务启动时若仍无有效元数据（如入队时获取失败），再 fetch 一次。
    if (existing && !hasValidInfo(existing.info)) {
      fetchAndFillInfo(id, existing.url);
    }
  });

  listen<any>("download-paused", (event) => {
    const id = String(event.payload?.task_id ?? "");
    if (id) patchTask(id, { status: "paused", speed: "" });
  });

  listen<any>("download-finished", (event) => {
    const p = event.payload ?? {};
    const id = String(p.task_id ?? "");
    if (!id) return;
    // 任务进入终态后从活跃列表移除（历史已由后端记录，「下载完成」板块
    // 通过 list_download_history 展示）。
    const done = state.queueTasks.find((x) => x.id === id);
    setState({ queueTasks: state.queueTasks.filter((x) => x.id !== id) });
    if (p.status === "completed") {
      const doneTitle =
        done?.info?.title || done?.title || done?.url || i18nT("gbar.completeBody");
      const filePath = typeof p.file_path === "string" ? p.file_path : "";
      toast.success(
        createElement(
          "div",
          { className: "flex items-center gap-1.5 min-w-0 w-full" },
          createElement("span", { className: "shrink-0" }, i18nT("gbar.complete")),
          createElement(
            "button",
            {
              className:
                "truncate min-w-0 text-left text-zinc-700 cursor-pointer underline-offset-2 hover:underline hover:text-blue-600",
              title: i18nT("gbar.completeOpen"),
              onClick: () => {
                if (!filePath) return;
                openFilePath(filePath).catch((e: any) =>
                  toast.error(i18nT("video.openPathFail", { err: e }))
                );
              },
            },
            doneTitle
          )
        ),
        { id: "download-global" }
      );
      notify(i18nT("gbar.complete"), doneTitle);
    } else if (p.status === "failed") {
      const msg = friendlyErrorMessage(p.error);
      toast.error(i18nT("gbar.failed", { msg }), { id: "download-global" });
      notify(
        i18nT("gbar.failedTitle"),
        `${done?.title || done?.url || i18nT("common.video")}：${msg}`
      );
    }
  });

  // Sync the queue snapshot so tasks survive a frontend reload (e.g. dev
  // hot-reload / page remount).
  queueStatus()
    .then((items) => {
      if (items.length === 0) return;
      // 恢复时回填卡片信息（保存进度退出后重启不丢信息）。
      // 顺序直接采用后端返回的顺序（running → paused → queued），
      // queued 段即实际下载顺序 —— 不按 seq 排序，避免 pause/resume 后
      // 显示顺序与下载顺序脱节。
      const itemsOrdered = [...items];
      setState({
        queueTasks: itemsOrdered.map((it) => ({
          id: it.task_id,
          url: it.url,
          title: it.title,
          status: it.status,
          percent: 0,
          speed: "",
          stage: "",
          // 后端持久化的卡片信息（保存进度退出重启后恢复）。
          info: (it.info as TaskInfo | undefined) ?? undefined,
          source: typeof it.source === "string" ? it.source : undefined,
        })),
      });
    })
    .catch(() => {});
}

// ==================== Queue actions ====================

/** Enqueue a download into the queue. Returns the task id. When the config
 *  already carries video metadata (single-download flow), it is attached to
 *  the task so cards can show cover / author / stats immediately. */
export async function enqueueDownloadGlobal(
  cfg: DownloadConfig,
  opts?: { title?: string | null; autoStart?: boolean; source?: string }
): Promise<string> {
  const info: TaskInfo = {
    thumbnail: cfg.thumbnail ?? null,
    uploader: cfg.uploader ?? null,
    duration: cfg.duration ?? 0,
    view_count: cfg.view_count ?? 0,
    like_count: cfg.like_count ?? 0,
    title: cfg.title ?? null,
  };
  // 把 info 一并提交给后端持久化（随 queue.json 保存，重启后可恢复）。
  const id = await enqueueDownload(
    cfg,
    opts?.title ?? null,
    opts?.autoStart,
    info,
    opts?.source
  );
  setState({
    queueTasks: state.queueTasks.map((t) =>
      t.id === id ? { ...t, info, source: opts?.source ?? t.source } : t
    ),
  });
  return id;
}

/** Cancel a queued / running task by id. */
export function cancelQueueTaskGlobal(taskId: string) {
  cancelQueueTask(taskId).catch(() => {});
}

/** Remove a task from the queue list. Active tasks (queued / paused /
 *  downloading) are also cancelled on the backend first so they don't get
 *  resurrected by a later refresh; finished tasks are removed locally only. */
export function removeQueueTaskGlobal(id: string) {
  const t = state.queueTasks.find((x) => x.id === id);
  if (
    t &&
    (t.status === "queued" || t.status === "paused" || t.status === "downloading")
  ) {
    cancelQueueTask(id).catch(() => {});
  }
  setState({ queueTasks: state.queueTasks.filter((x) => x.id !== id) });
}

/** Clear the task list: cancel queued tasks, drop finished records, keep
 *  currently running tasks (they continue and are still shown). */
export function clearQueueTasksGlobal() {
  clearDownloadQueue().catch(() => {});
  setState({
    queueTasks: state.queueTasks.filter((t) => t.status === "downloading"),
  });
}

/** Move a queued task to a new position (0 = top). Refreshes the list so the
 *  seq-based ordering reflects the reordered queue. */
export function reorderQueueTaskGlobal(taskId: string, newIndex: number) {
  reorderQueueTask(taskId, newIndex)
    .catch(() => {})
    .finally(() => refreshQueueGlobal());
}

/** Sync the frontend task list with the backend snapshot. Preserves the
 *  progress/speed/info of tasks already shown; updates status/url/title. */
export async function refreshQueueGlobal() {
  try {
    const items = await queueStatus();
    // 采用后端返回顺序（running → paused → queued，queued 即下载顺序），
    // 与"全部暂停→全部开始"后的实际下载顺序保持一致。
    const itemsOrdered = [...items];
    setState({
      queueTasks: itemsOrdered.map((it) => {
        const existing = state.queueTasks.find((t) => t.id === it.task_id);
        return {
          id: it.task_id,
          url: it.url,
          title: it.title,
          status: it.status,
          percent: existing?.percent ?? 0,
          speed: existing?.speed ?? "",
          stage: existing?.stage ?? "",
          info:
            existing?.info ?? ((it.info as TaskInfo | undefined) ?? undefined),
          source:
            existing?.source ??
            (typeof it.source === "string" ? it.source : undefined),
        };
      }),
    });
  } catch {
    // ignore
  }
}

/** Cancel ALL active tasks (queued / paused / running). Finished downloads in
 *  the history are untouched; running tasks emit download-finished(cancelled). */
export function cancelAllActiveGlobal() {
  cancelAllTasks()
    .catch(() => {})
    .finally(() => refreshQueueGlobal());
  setState({
    queueTasks: state.queueTasks.filter(
      (t) => t.status === "completed" || t.status === "failed"
    ),
  });
}

/** Pause a single task (queued task stops waiting; running task is killed and
 *  its cache kept for resume). */
export function pauseQueueTaskGlobal(taskId: string) {
  pauseQueueTask(taskId)
    .catch(() => {})
    .finally(() => refreshQueueGlobal());
}

/** 重新获取某任务的信息（信息获取失败后手动重试）。
 *  成功 → 恢复排队等待下载；失败 → 保持暂停 + infoFailed，可继续重试。 */
export function refetchTaskInfoGlobal(taskId: string) {
  const task = state.queueTasks.find((t) => t.id === taskId);
  if (!task) return;
  patchTask(taskId, { infoFailed: undefined });
  fetchAndFillInfoAsync(taskId, task.url).then(() => {
    const t = state.queueTasks.find((x) => x.id === taskId);
    if (t && hasValidInfo(t.info)) {
      // 信息就绪 → 恢复排队（resume_task 内部会 pump，可能立即启动下载）
      resumeQueueTask(taskId)
        .catch(() => {})
        .finally(() => refreshQueueGlobal());
    } else {
      // 仍失败：fetchAndFillInfoAsync 已重新标记 infoFailed + 保持暂停
      refreshQueueGlobal();
    }
  });
}

/** Resume a paused task (downloads continue from the kept .part cache). */
export function resumeQueueTaskGlobal(taskId: string) {
  resumeQueueTask(taskId)
    .catch(() => {})
    .finally(() => refreshQueueGlobal());
}

/** 批量入队后调用：逐任务获取信息，每完成一个即按并发尝试启动下载。
 *  @param expectedCount 本次成功入队的任务数（用于等待事件全部到达）。 */
export function prepareQueue(expectedCount?: number) {
  infoFetchState.expected = expectedCount ?? 0;
  runInfoFetch();
}

/** Pause every active task (and the info-fetch phase). */
export function pauseAllGlobal() {
  pauseInfoFetch();
  pauseAllTasks()
    .catch(() => {})
    .finally(() => refreshQueueGlobal());
}

/** Resume every paused task (and continue the info-fetch phase). */
export function resumeAllGlobal() {
  resumeInfoFetch();
  resumeAllTasks()
    .catch(() => {})
    .finally(() => refreshQueueGlobal());
}
