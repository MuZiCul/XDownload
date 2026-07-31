import { useEffect, useState, useCallback } from "react";
import { checkYtdlp, checkFfmpeg, downloadYtDlp, downloadFfmpeg } from "../lib/bindings";
import type { ToolStatus } from "../lib/types";

// Shared singleton state — only fetched once per app lifetime
let cachedYtStatus: ToolStatus | null = null;
let cachedFfStatus: ToolStatus | null = null;
let pendingPromise: Promise<ToolStatus[]> | null = null;

const listeners = new Set<() => void>();

function notify() {
  listeners.forEach((fn) => fn());
}

function fetchAll(): Promise<ToolStatus[]> {
  if (!pendingPromise) {
    pendingPromise = Promise.all([checkYtdlp(), checkFfmpeg()]);
    pendingPromise.then(
      ([yt, ff]) => {
        cachedYtStatus = yt;
        cachedFfStatus = ff;
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
    // Force re-check (e.g. after downloading)
    pendingPromise = null;
    cachedYtStatus = null;
    cachedFfStatus = null;
    const [yt, ff] = await Promise.all([checkYtdlp(), checkFfmpeg()]);
    cachedYtStatus = yt;
    cachedFfStatus = ff;
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
    refresh,
    download,
  };
}
