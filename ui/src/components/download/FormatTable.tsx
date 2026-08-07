import type { Format } from "../../lib/types";
import { useI18n } from "../../lib/i18n";

type Props = {
  formats: Format[];
};

function hasVideo(f: Format): boolean {
  return !!f.vcodec && f.vcodec !== "none";
}

function formatRes(f: Format): string {
  if (f.resolution) return f.resolution;
  if (f.height) return `${f.width ?? "?"}x${f.height}`;
  if (!hasVideo(f)) return "audio only";
  return "video";
}

function fileSizeStr(f: Format): string {
  const bytes = f.filesize ?? f.filesize_approx ?? 0;
  if (bytes <= 0) return "?";
  const b = bytes;
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  if (b < 1024 * 1024 * 1024) return `${(b / (1024 * 1024)).toFixed(1)} MB`;
  return `${(b / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Read-only list of formats parsed from the URL. Downloads always use the
 * "best" format — this table is informational only.
 */
export default function FormatTable({ formats }: Props) {
  const { t } = useI18n();
  if (formats.length === 0) {
    return (
      <div className="section-card">
        <div className="section-title">{t("format.title")}</div>
        <p className="text-xs text-gray-400 text-center py-4">—</p>
      </div>
    );
  }

  return (
    <div className="section-card">
      <div className="section-title">{t("format.title")}</div>
      <div className="max-h-[220px] overflow-auto border border-gray-200 rounded">
        <table className="w-full format-table">
          <thead>
            <tr>
              <th>{t("format.id")}</th>
              <th>{t("format.ext")}</th>
              <th>{t("format.res")}</th>
              <th>{t("format.size")}</th>
            </tr>
          </thead>
          <tbody>
            {formats.map((f) => {
              const fmtId = f.format_id;
              const shortId =
                fmtId.length > 22 ? fmtId.substring(0, 19) + "..." : fmtId;
              return (
                <tr key={fmtId}>
                  <td className="font-mono">{shortId}</td>
                  <td>{f.ext ?? "?"}</td>
                  <td>{formatRes(f)}</td>
                  <td>{fileSizeStr(f)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
