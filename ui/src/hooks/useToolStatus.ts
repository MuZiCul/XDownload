import { useEffect, useState, useCallback } from "react";
import {
  checkYtdlp,
  checkFfmpeg,
  checkYtdlpUpdate,
  checkFfmpegUpdate,
  downloadYtDlp,
  downloadFfmpeg,
} from "../lib/bindings";
import type { ToolStatus } from "../lib/types";

// Shared singleton state — only fetched once per app lifetime
let cachedYtStatus: ToolStatus | null = null;
let cachedFfStatus: ToolStatus | null = null;
// Whether an update is available (from check_*_update, compares versions)
let cachedYtUpdate = false;
let cachedFfUpdate = false;
let pendingPromise: Promise<unknown> | null = null;

const listeners = new Set<() => void>();

function notify() {
  listeners.forEach((fn) => fn());
}

function fetchAll(): Promise<unknown> {
  if (!pendingPromise) {
    pendingPromise = Promise.all([
      checkYtdlp(),
      checkFfmpeg(),
      checkYtdlpUpdate().catch(() => null),
      checkFfmpegUpdate().catch(() => null),
    ]).then(
      ([yt, ff, ytUp, ffUp]) => {
        cachedYtStatus = yt as ToolStatus;
        cachedFfStatus = ff as ToolStatus;
        cachedYtUpdate = !!(ytUp as { has_update?: boolean } | null)?.has_update;
        cachedFfUpdate = !!(ffUp as { has_update?: boolean } | null)?.has_update;
        notify();
      },
      () => {
        pendingPromise = null; // allow retry on next access
      },
    );
  }
  return pendingPromise;
}

// Start fetching immediately when this module loads
fetchAll();

export function useToolStatus() {
  const [, forceUpdate] = useState(0);

  useEffect(() => {
    const listener = () => forceUpdate((n) => n + 1);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);

  const refresh = useCallback(async () => {
    // Force re-check (e.g. after downloading / pressing "check update").
    // NOTE: keep the cached values untouched until the new data arrives, so
    // the UI keeps showing the previous status while the check is in flight
    // instead of flickering to "not installed" (which happens when the cache
    // is cleared first and the fallback `{ available: false }` is rendered).
    pendingPromise = null;
    const [yt, ff, ytUp, ffUp] = await Promise.all([
      checkYtdlp(),
      checkFfmpeg(),
      checkYtdlpUpdate().catch(() => null),
      checkFfmpegUpdate().catch(() => null),
    ]);
    cachedYtStatus = yt;
    cachedFfStatus = ff;
    cachedYtUpdate = !!ytUp?.has_update;
    cachedFfUpdate = !!ffUp?.has_update;
    notify();
  }, []);

  const download = useCallback(
    async (tool: "yt-dlp" | "ffmpeg"): Promise<void> => {
      if (tool === "yt-dlp") {
        await downloadYtDlp();
      } else {
        await downloadFfmpeg();
      }
      await refresh();
    },
    [refresh],
  );

  return {
    ytStatus: cachedYtStatus ?? { available: false, version: null },
    ffStatus: cachedFfStatus ?? { available: false, version: null },
    hasYtUpdate: cachedYtUpdate,
    hasFfUpdate: cachedFfUpdate,
    refresh,
    download,
  };
}
