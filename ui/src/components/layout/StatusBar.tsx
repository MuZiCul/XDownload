import { useEffect, useState } from "react";
import { getProxyStatus } from "../../lib/bindings";
import type { ProxyStatus } from "../../lib/types";
import { useToolStatus } from "../../hooks/useToolStatus";
import { Activity } from "lucide-react";

export default function StatusBar() {
  const { ytStatus, ffStatus } = useToolStatus();
  const [pxStatus, setPxStatus] = useState<ProxyStatus | null>(null);

  const refreshProxyStatus = () => {
    getProxyStatus().then(setPxStatus).catch(() => {});
  };

  useEffect(() => {
    refreshProxyStatus();
    const handler = () => refreshProxyStatus();
    window.addEventListener("config-applied", handler);
    return () => window.removeEventListener("config-applied", handler);
  }, []);

  return (
    <footer className="flex items-center gap-4 px-4 py-1.5 bg-white border-t border-zinc-200/80 text-[11px] text-zinc-500 shrink-0">
      <StatusBadge
        label="yt-dlp"
        ok={ytStatus.available}
        detail={ytStatus.version ?? "?"}
      />
      <StatusBadge
        label="ffmpeg"
        ok={ffStatus.available}
        detail={ffStatus.version ?? (ffStatus.available ? "?" : "N/A")}
      />
      <div className="flex-1" />
      <StatusBadge
        label="proxy"
        ok={!!pxStatus?.enabled}
        detail={pxStatus?.enabled ? pxStatus.proxy_string : "off"}
      />
    </footer>
  );
}

function StatusBadge({ label, ok, detail }: { label: string; ok: boolean; detail: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="text-zinc-400">{label}</span>
      <span className="flex items-center gap-1">
        <span className={`inline-block w-1.5 h-1.5 rounded-full ${ok ? "bg-emerald-500" : "bg-zinc-300"}`} />
        <span className={ok ? "text-zinc-700 font-medium" : "text-zinc-400"}>
          {detail}
        </span>
      </span>
    </div>
  );
}
