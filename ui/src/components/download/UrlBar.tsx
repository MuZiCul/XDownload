import { Download, ClipboardPaste } from "lucide-react";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { toast } from "sonner";
import { useI18n } from "../../lib/i18n";

type Props = {
  url: string;
  onUrlChange: (v: string) => void;
  onFetch: (url: string) => void;
  isLoading: boolean;
};

export default function UrlBar({ url, onUrlChange, onFetch, isLoading }: Props) {
  const { t } = useI18n();

  const handleFetch = () => {
    const trimmed = url.trim();
    if (trimmed) onFetch(trimmed);
  };

  const handlePaste = async () => {
    try {
      let text = "";
      try {
        text = await readText();
      } catch {
        text = await navigator.clipboard.readText();
      }
      if (text) {
        onUrlChange(text.trim());
      } else {
        toast.info(t("url.clipboard.empty"));
      }
    } catch {
      toast.error(t("url.clipboard.fail"));
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleFetch();
  };

  return (
    <div className="flex items-center gap-2 mb-3 bg-white border border-zinc-200 rounded-xl px-4 py-2.5 shadow-sm">
      <input
        type="text"
        value={url}
        onChange={(e) => onUrlChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={t("url.placeholder")}
        className="flex-1 border-none bg-transparent focus:outline-none text-sm px-0"
      />
      <button className="btn flex items-center gap-1.5" onClick={handlePaste} disabled={isLoading}>
        <ClipboardPaste size={13} />
        {t("url.paste")}
      </button>
      <button
        className="btn btn-primary flex items-center gap-1.5 px-4"
        onClick={handleFetch}
        disabled={isLoading}
      >
        <Download size={13} />
        {t("url.fetch")}
      </button>
    </div>
  );
}
