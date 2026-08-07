import { Download, Settings, Info, ScrollText, History } from "lucide-react";
import { useI18n } from "../../lib/i18n";

type TabId = "download" | "settings" | "history" | "about" | "disclaimer";

type Props = {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
};

const tabs: { id: TabId; labelKey: string; Icon: typeof Download }[] = [
  { id: "download", labelKey: "tab.download", Icon: Download },
  { id: "settings", labelKey: "tab.settings", Icon: Settings },
  { id: "history", labelKey: "tab.history", Icon: History },
  { id: "about", labelKey: "tab.about", Icon: Info },
  { id: "disclaimer", labelKey: "tab.disclaimer", Icon: ScrollText },
];

export default function TabBar({ activeTab, onTabChange }: Props) {
  const { t } = useI18n();
  return (
    <nav className="flex items-center bg-white border-b border-zinc-200 px-3 shrink-0" role="tablist">
      <div className="flex items-center gap-0.5">
        {tabs.map(({ id, labelKey, Icon }) => (
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
            {t(labelKey)}
          </button>
        ))}
      </div>
    </nav>
  );
}
