import { useEffect, useRef } from "react";
import { Link2, ExternalLink } from "lucide-react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";

export interface ContextMenuItem {
  key: string;
  label: string;
  icon?: React.ReactNode;
  onClick: () => void;
}

/** 自定义右键菜单：按鼠标位置显示，点击外部 / 滚动 / Esc 关闭。 */
export default function ContextMenu({
  x,
  y,
  items,
  onClose,
}: {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const close = () => onClose();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onDocClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener("mousedown", onDocClick);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDocClick);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="fixed z-[60] min-w-[160px] bg-white/90 backdrop-blur-md border border-zinc-200 rounded-lg shadow-xl py-1"
      style={{ left: x, top: y }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item) => (
        <button
          key={item.key}
          className="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-zinc-700 hover:bg-zinc-100 text-left"
          onClick={() => {
            item.onClick();
            onClose();
          }}
        >
          {item.icon}
          {item.label}
        </button>
      ))}
    </div>
  );
}

/** 快捷构造一个「复制链接」菜单项。复制是异步的：成功后才回调
 *  `onCopyDone`，失败回调 `onCopyError`（避免"显示成功但剪贴板没内容"）。 */
export function copyLinkItem(
  label: string,
  url: string,
  onCopyDone: () => void,
  onCopyError?: () => void
): ContextMenuItem {
  return {
    key: "copy-link",
    label,
    icon: <Link2 size={13} className="text-zinc-400" />,
    onClick: async () => {
      try {
        await writeText(url);
        onCopyDone();
      } catch {
        onCopyError?.();
      }
    },
  };
}

/** 快捷构造一个「在浏览器打开」菜单项。 */
export function openLinkItem(label: string, url: string): ContextMenuItem {
  return {
    key: "open-link",
    label,
    icon: <ExternalLink size={13} className="text-zinc-400" />,
    onClick: () => {
      if (url) openUrl(url).catch(() => {});
    },
  };
}
