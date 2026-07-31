import { Download, ClipboardPaste } from "lucide-react";
import { useState } from "react";

type Props = {
  onFetch: (url: string) => void;
  isLoading: boolean;
};

export default function UrlBar({ onFetch, isLoading }: Props) {
  const [url, setUrl] = useState("");

  const handleFetch = () => {
    const trimmed = url.trim();
    if (trimmed) onFetch(trimmed);
  };

  const handlePaste = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setUrl(text.trim());
        onFetch(text.trim());
      }
    } catch {}
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleFetch();
  };

  return (
    <div className="flex items-center gap-2 mb-3 bg-white border border-zinc-200 rounded-xl px-4 py-2.5 shadow-sm">
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="粘贴 X/Twitter 视频链接..."
        className="flex-1 border-none bg-transparent focus:outline-none text-sm px-0"
      />
      <button className="btn flex items-center gap-1.5" onClick={handlePaste} disabled={isLoading}>
        <ClipboardPaste size={13} />
        粘贴
      </button>
      <button
        className="btn btn-primary flex items-center gap-1.5 px-4"
        onClick={handleFetch}
        disabled={isLoading}
      >
        <Download size={13} />
        获取信息
      </button>
    </div>
  );
}
