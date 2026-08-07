import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { toast } from "sonner";
import {
  startDownload,
  cancelDownload,
  isDownloading,
} from "./bindings";
import type { DownloadConfig } from "./types";
import { friendlyErrorMessage } from "./errorMessages";
import { t as i18nT } from "./i18n";

export interface DlProgress {
  percent: number;
  speed: string;
  eta: string;
  status: string; // downloading | merging | postprocess | finished
  stage: string; // video | audio | merge | ""
}

export interface DownloadState {
  downloading: boolean;
  progress: DlProgress | null;
  title: string | null;
  url: string | null;
  error: string | null;
  completed: boolean;
  completedAt: number | null;
}

const initialState: DownloadState = {
  downloading: false,
  progress: null,
  title: null,
  url: null,
  error: null,
  completed: false,
  completedAt: null,
};

// Module-level state so it survives component unmounts (tab switches).
let state: DownloadState = initialState;
const listeners = new Set<() => void>();

function setState(partial: Partial<DownloadState>) {
  const next = { ...state, ...partial };
  // "Completed" and "error" are mutually exclusive (error wins): a stale
  // error left over from an earlier failure must not render next to the
  // green completion check, and vice versa. Centralizing this here covers
  // every update path (events, promise safety nets, recovery).
  if (next.completed) next.error = null;
  if (next.error) next.completed = false;
  state = next;
  listeners.forEach((l) => l());
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function getSnapshot(): DownloadState {
  return state;
}

/** Subscribe a component to the global download state. */
export function useDownloadStore(): DownloadState {
  return useSyncExternalStore(subscribe, getSnapshot);
}

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
 * Register global download event listeners (once) and restore the running
 * state. Must be called once, e.g. from App's useEffect.
 */
export function initDownloadStore() {
  if (initialized) return;
  initialized = true;

  listen<any>("download-progress", (event) => {
    const p = event.payload;
    const pct = parseFloat(String(p.percent ?? "").replace("%", "")) || 0;
    setState({
      downloading: true,
      error: null,
      completed: false,
      progress: {
        percent: Math.min(Math.max(pct, 0), 100),
        speed: p.speed ?? "",
        eta: p.eta ?? "",
        status: p.status ?? "downloading",
        stage: p.stage ?? "",
      },
    });
  });

  listen("download-complete", () => {
    setState({
      downloading: false,
      completed: true,
      completedAt: Date.now(),
      error: null,
    });
    toast.success(i18nT("gbar.complete"), { id: "download-global" });
    notify(
      i18nT("gbar.complete"),
      state.title || state.url || i18nT("gbar.completeBody")
    );
  });

  listen("download-error", (event) => {
    const msg = friendlyErrorMessage(event.payload);
    setState({ downloading: false, error: msg, completed: false });
    toast.error(i18nT("gbar.failed", { msg }), { id: "download-global" });
    notify(
      i18nT("gbar.failedTitle"),
      `${state.title || state.url || i18nT("common.video")}：${msg}`
    );
  });

  // Recover a running download (e.g. dev hot-reload / window reopen mid-task).
  isDownloading()
    .then((busy) => {
      if (busy) setState({ downloading: true });
    })
    .catch(() => {});
}

/** Start a download through the global store. Returns whether it succeeded. */
export async function startDownloadGlobal(
  cfg: DownloadConfig,
  opts?: { title?: string | null }
): Promise<boolean> {
  setState({
    downloading: true,
    progress: { percent: 0, speed: "", eta: "", status: "downloading", stage: "" },
    title: opts?.title ?? null,
    url: cfg.url,
    error: null,
    completed: false,
    completedAt: null,
  });
  try {
    const ok = await startDownload(cfg);
    if (ok) {
      // The download-complete event usually fires first; this is a safety net.
      setState({ downloading: false, completed: true, completedAt: Date.now() });
    } else {
      setState({ downloading: false, error: i18nT("gbar.failedTitle") });
      toast.error(i18nT("gbar.failedTitle"), { id: "download-global" });
    }
    return ok;
  } catch (err) {
    const msg = friendlyErrorMessage(err);
    setState({ downloading: false, error: msg });
    toast.error(i18nT("gbar.failed", { msg }), {
      id: "download-global",
    });
    throw err;
  }
}

export function cancelDownloadGlobal() {
  cancelDownload().catch(() => {});
}

/** Hide the finished / error banner in the global download bar. */
export function dismissDownloadResult() {
  setState({
    completed: false,
    error: null,
    completedAt: null,
    progress: null,
    title: null,
    url: null,
  });
}
