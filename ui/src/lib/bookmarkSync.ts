import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { syncBookmarksPreview, confirmBookmarksEnqueue } from "./bindings";
import { prepareQueue } from "./downloadStore";
import type { BookmarksSyncPreview, BookmarksVideoItem } from "./types";
import { toast } from "sonner";
import { t } from "./i18n";

/**
 * 书签同步全局状态（模块级 + useSyncExternalStore）。
 *
 * 因为 SettingsPage 在切 tab 时会卸载，同步中状态必须提升到模块级，
 * 否则「同步中」会被误认为结束。同步期间由 App 根部的
 * `BookmarkSyncModal` 显示一个无法关闭的模态，步骤文案来自后端
 * `bookmark-sync-progress` 事件。
 *
 * 阶段流转：
 *   idle → syncing → preview（有可入队视频）→ idle（确认/取消）
 *   syncing → error（失败）→ idle（关闭）
 *   syncing → idle（无视频书签，toast 提示）
 */
export type BookmarkSyncPhase = "idle" | "syncing" | "preview" | "error";
export type BookmarkSyncStepKey = "cookies" | "fetch" | "persist" | "diff";

interface BookmarkSyncState {
  phase: BookmarkSyncPhase;
  /** 后端 progress 事件的最新步骤（仅 syncing 阶段有效）。 */
  step: BookmarkSyncStepKey | null;
  preview: BookmarksSyncPreview | null;
  error: { queryId: boolean; msg: string } | null;
}

const initialState: BookmarkSyncState = {
  phase: "idle",
  step: null,
  preview: null,
  error: null,
};

let state: BookmarkSyncState = initialState;
const listeners = new Set<() => void>();

function setState(partial: Partial<BookmarkSyncState>) {
  state = { ...state, ...partial };
  listeners.forEach((l) => l());
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function getSnapshot(): BookmarkSyncState {
  return state;
}

/** React hook: subscribe to the global bookmark sync state. */
export function useBookmarkSync(): BookmarkSyncState {
  return useSyncExternalStore(subscribe, getSnapshot);
}

/**
 * Trigger a manual bookmark sync. Safe to call from any tab; the global modal
 * keeps the "syncing" state visible even when the settings tab is unmounted.
 */
export async function runBookmarkSync(): Promise<void> {
  if (state.phase === "syncing") return;
  setState({ phase: "syncing", step: "cookies", preview: null, error: null });
  try {
    const result = await syncBookmarksPreview();
    if (result.video_items.length === 0) {
      if (result.new_count === 0) {
        toast.success(t("bookmarks.noNew"));
      } else {
        toast.info(t("bookmarks.noVideo", { count: result.new_count }));
      }
      setState({ phase: "idle", step: null, preview: null });
      return;
    }
    setState({ phase: "preview", step: null, preview: result });
  } catch (err: any) {
    const msg = String(err);
    // 后端对 queryId 失效的错误带 "queryId" 关键词，据此展示专门引导。
    setState({
      phase: "error",
      step: null,
      error: { queryId: msg.includes("queryId"), msg },
    });
  }
}

/**
 * Stage 2 confirm: enqueue the user's selected bookmarks, then close the
 * modal. Returns false when the modal should stay open (enqueue failure).
 */
export async function confirmBookmarkSelection(
  items: BookmarksVideoItem[]
): Promise<boolean> {
  try {
    const result = await confirmBookmarksEnqueue(items);
    toast.success(t("bookmarks.enqueued", { count: result.queued_count }));
    setState({ phase: "idle", step: null, preview: null });
    if (result.queued_count > 0) {
      // 两阶段信息获取：任务先排队（auto_start=false），前端逐任务 fetch
      // 视频信息 → 每完成一个按并发启动下载，卡片始终有封面/标题/作者。
      prepareQueue(result.queued_count);
    }
    return true;
  } catch (err: any) {
    toast.error(t("bookmarks.enqueueFail", { err }));
    return false;
  }
}

/** Close the sync modal (preview/cancel or dismiss an error). */
export function dismissBookmarkSync(): void {
  setState({ phase: "idle", step: null, preview: null, error: null });
}

let initialized = false;

/** Register the backend progress listener once (call from App on mount). */
export function initBookmarkSync(): void {
  if (initialized) return;
  initialized = true;
  listen<any>("bookmark-sync-progress", (event) => {
    if (state.phase !== "syncing") return;
    const step = String(event.payload?.step ?? "");
    if (step === "cookies" || step === "fetch" || step === "persist" || step === "diff") {
      setState({ step });
    }
  });
}
