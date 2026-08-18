import { useState } from "react";
import type { ReactNode } from "react";
import { HelpCircle, X } from "lucide-react";
import { useI18n } from "../../lib/i18n";

/**
 * 设置卡片标题：标题右侧带问号图标，点击后弹出拟态窗显示说明。
 * 外层保持 `.section-title` 的块级样式不变；`title` 支持任意节点
 * （如标题内嵌状态徽标）。
 */
export default function SectionTitle({
  title,
  tip,
}: {
  title: ReactNode;
  tip?: string;
}) {
  const { t } = useI18n();
  const [showTip, setShowTip] = useState(false);

  return (
    <div className="section-title">
      <span className="inline-flex items-center gap-1.5">
        {title}
        {tip && (
          <HelpCircle
            size={14}
            className="text-zinc-400 hover:text-blue-500 transition-colors cursor-pointer"
            onClick={(e) => {
              e.stopPropagation();
              setShowTip(true);
            }}
          />
        )}
      </span>

      {showTip && (
        <div className="dialog-overlay" onClick={() => setShowTip(false)}>
          <div
            className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[90] w-[420px] max-w-[92vw] bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/50 p-5"
            onClick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
          >
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-semibold text-zinc-900">
                {title}
              </span>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={() => setShowTip(false)}
              >
                <X size={16} />
              </button>
            </div>
            <div className="text-xs text-zinc-600 leading-relaxed">
              {(tip ?? "").split("\n").map((line, i) => (
                // 悬挂缩进：带「·」的条目行用 pl-3 -indent-3（续行对齐到点后文字）；
                // 总述首行（index 0，无点）不缩进，其软换行续行与首行文字起点对齐。
                <div key={i} className={i === 0 ? "" : "pl-3 -indent-3"}>
                  {line}
                </div>
              ))}
            </div>
            <div className="flex justify-end mt-4">
              <button className="btn" onClick={() => setShowTip(false)}>
                {t("common.close")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
