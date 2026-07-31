import type { VideoInfo } from "../../lib/types";

function formatDuration(seconds: number): string {
  if (!seconds || seconds <= 0) return "?";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function formatNumber(n: number): string {
  if (n >= 100_000_000) return `${(n / 100_000_000).toFixed(1)}亿`;
  if (n >= 10_000) return `${(n / 10_000).toFixed(1)}万`;
  return n.toLocaleString();
}

type Props = {
  info: VideoInfo | null;
};

export default function VideoInfoCard({ info }: Props) {
  return (
    <div className="section-card">
      <div className="section-title">视频信息</div>

      {/* Thumbnail + metadata grid */}
      <div className="flex gap-3">
        {info?.thumbnail && (
          <img
            src={info.thumbnail}
            alt="thumbnail"
            className="w-28 h-[72px] object-cover rounded-lg border border-zinc-200 shrink-0"
          />
        )}
        <div className="flex-1 min-w-0">
          <h3 className="text-[13px] font-semibold text-zinc-900 leading-snug line-clamp-2 mb-2">
            {info?.title ?? "—"}
          </h3>
          <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs">
            <InfoRow label="作者" value={info?.uploader ?? "—"} />
            <InfoRow label="时长" value={info ? formatDuration(info.duration) : "—"} />
            <InfoRow label="播放" value={info ? formatNumber(info.view_count) : "—"} />
            <InfoRow label="点赞" value={info ? formatNumber(info.like_count) : "—"} />
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
