import { Download, Settings, Info } from "lucide-react";

type Props = {
  activeTab: "download" | "settings" | "about";
  onTabChange: (tab: "download" | "settings" | "about") => void;
};

const tabs = [
  { id: "download" as const, label: "下载", Icon: Download },
  { id: "settings" as const, label: "设置", Icon: Settings },
  { id: "about" as const, label: "关于", Icon: Info },
];

export default function TabBar({ activeTab, onTabChange }: Props) {
  return (
    <nav className="flex items-center bg-white border-b border-zinc-200 px-3 shrink-0" role="tablist">
      <div className="flex items-center gap-0.5">
        {tabs.map(({ id, label, Icon }) => (
          <button
            key={id}
            role="tab"
            aria-selected={activeTab === id}
            onClick={() => onTabChange(id)}
            className={`relative flex items-center gap-2 px-5 py-2.5 text-[13px] font-medium rounded-lg transition-all duration-150 ${
              activeTab === id
                ? "text-blue-700 bg-blue-50/80"
                : "text-zinc-500 hover:text-zinc-800 hover:bg-zinc-100"
            }`}
          >
            <Icon size={15} />
            {label}
          </button>
        ))}
      </div>
    </nav>
  );
}
