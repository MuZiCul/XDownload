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
import type { YtdlpUpdateResult, FfmpegUpdateResult } from "../lib/bindings";

// Shared singleton state — only fetched once per app lifetime
let cachedYtStatus: ToolStatus | null = null;
let cachedFfStatus: ToolStatus | null = null;
// Full update-check results (carry local/latest versions, not_installed, error)
let cachedYtUpdate: YtdlpUpdateResult | null = null;
let cachedFfUpdate: FfmpegUpdateResult | null = null;
let pendingPromise: Promise<unknown> | null = null;

const listeners = new Set<() => void>();

function notify() {
  listeners.forEach((fn) => fn());
}

async function checkAll(forceFfRefresh = false): Promise<{
  yt: ToolStatus;
  ff: ToolStatus;
  ytUp: YtdlpUpdateResult | null;
  ffUp: FfmpegUpdateResult | null;
}> {
  // Sequential availability check first, then update checks that REUSE the
  // already-fetched local version — this avoids spawning a second yt-dlp
  // process at startup (which both slowed things down and doubled the
  // chance of a cold-start timeout).
  const yt = await checkYtdlp();
  const ff = await checkFfmpeg();
  const [ytUp, ffUp] = await Promise.all([
    checkYtdlpUpdate(yt.version ?? undefined).catch(() => null),
    // 启动/自动检查走 24h 缓存；用户手动点「检查更新」时 forceFfRefresh=true 强制联网刷新。
    checkFfmpegUpdate(forceFfRefresh ? true : undefined).catch(() => null),
  ]);
  return { yt, ff, ytUp, ffUp };
}

function fetchAll(): Promise<unknown> {
  if (!pendingPromise) {
    pendingPromise = (async () => {
      const res = await checkAll();
      cachedYtStatus = res.yt;
      cachedFfStatus = res.ff;
      cachedYtUpdate = res.ytUp;
      cachedFfUpdate = res.ffUp;
      notify();
    })().catch(() => {
      pendingPromise = null; // allow retry on next access
    });
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

  const refresh = useCallback(async (forceFfRefresh = false) => {
    // Force re-check (e.g. after downloading / pressing "check update").
    // NOTE: keep the cached values untouched until the new data arrives, so
    // the UI keeps showing the previous status while the check is in flight
    // instead of flickering to "not installed" (which happens when the cache
    // is cleared first and the fallback `{ available: false }` is rendered).
    pendingPromise = null;
    const res = await checkAll(forceFfRefresh);
    cachedYtStatus = res.yt;
    cachedFfStatus = res.ff;
    cachedYtUpdate = res.ytUp;
    cachedFfUpdate = res.ffUp;
    notify();
    return res;
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
    ytUpdate: cachedYtUpdate,
    ffUpdate: cachedFfUpdate,
    hasYtUpdate: !!cachedYtUpdate?.has_update,
    hasFfUpdate: !!cachedFfUpdate?.has_update,
    refresh,
    download,
  };
}
