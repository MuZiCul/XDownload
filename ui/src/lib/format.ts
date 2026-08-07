/** Shared display formatters used by both the download page and history page. */

export function formatDuration(seconds: number): string {
  if (!seconds || seconds <= 0) return "?";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function formatNumber(
  n: number,
  t: (key: string) => string
): string {
  if (n >= 100_000_000) return `${(n / 100_000_000).toFixed(1)}${t("num.billion")}`;
  if (n >= 10_000) return `${(n / 10_000).toFixed(1)}${t("num.tenThousand")}`;
  return n.toLocaleString();
}

export function formatDateTime(ts: number): string {
  if (!ts) return "—";
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}
