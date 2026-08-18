import { useEffect, useMemo, useRef, useState } from "react";
import {
  BarChart3,
  Download,
  HardDrive,
  CheckCircle2,
  CalendarDays,
  Clock,
  X,
  RefreshCw,
} from "lucide-react";
import { useI18n } from "../../lib/i18n";
import { getDownloadStats } from "../../lib/bindings";
import type { DownloadStats } from "../../lib/types";
import { loadEcharts } from "../../lib/echartsLoader";

/** 苹果官网风格统计页：大数字 + ECharts 图表。 */
export default function StatisticsPage({ onClose }: { onClose: () => void }) {
  const { t } = useI18n();
  const [stats, setStats] = useState<DownloadStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [echartsReady, setEchartsReady] = useState(false);

  const load = async (showLoading = true) => {
    if (showLoading) setLoading(true);
    setError(null);
    try {
      const s = await getDownloadStats();
      if (!s.ok) throw new Error(s.error || "stats failed");
      setStats(s);
    } catch (e: any) {
      setError(String(e?.message ?? e));
    } finally {
      if (showLoading) setLoading(false);
    }
  };

  useEffect(() => {
    load();
    // 加载 ECharts（本地文件优先，CDN 回退）。
    loadEcharts()
      .then(() => setEchartsReady(true))
      .catch((e) => setError(String(e?.message ?? e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const hero = stats?.hero;
  const usageDays = useMemo(() => {
    if (!hero || hero.first_at <= 0) return 0;
    return Math.max(1, Math.floor((hero.last_at - hero.first_at) / 86400) + 1);
  }, [hero]);

  return (
    <div className="fixed inset-0 z-40 overflow-y-auto bg-[#f5f5f7]">
      {/* 顶部条 */}
      <div className="sticky top-0 z-10 bg-[#f5f5f7]/90 backdrop-blur-xl border-b border-black/5">
        <div className="max-w-5xl mx-auto px-8 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2 text-zinc-800">
            <BarChart3 size={16} />
            <span className="text-sm font-semibold">{t("stats.title")}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <button
              className="p-1.5 rounded-full text-zinc-500 hover:text-zinc-800 hover:bg-black/5 transition-colors"
              onClick={() => load()}
              title={t("stats.refresh")}
            >
              <RefreshCw size={15} className={loading ? "animate-spin" : ""} />
            </button>
            <button
              className="p-1.5 rounded-full text-zinc-500 hover:text-zinc-800 hover:bg-black/5 transition-colors"
              onClick={onClose}
              title={t("common.close")}
            >
              <X size={18} />
            </button>
          </div>
        </div>
      </div>

      <div className="max-w-5xl mx-auto px-8 pb-16">
        {/* Hero：苹果风格大标题 */}
        <div className="pt-10 pb-6">
          <h1 className="text-4xl md:text-5xl font-bold tracking-tight text-zinc-900">
            {t("stats.heroTitle")}
          </h1>
          <p className="mt-2 text-lg text-zinc-500">{t("stats.heroSubtitle")}</p>
        </div>

        {loading && !stats ? (
          <div className="py-24 text-center text-sm text-zinc-400">
            {t("stats.loading")}
          </div>
        ) : error && !stats ? (
          <div className="py-24 text-center">
            <p className="text-sm text-red-500">{error}</p>
            <button className="btn mt-4" onClick={() => load()}>
              {t("stats.retry")}
            </button>
          </div>
        ) : stats && hero && hero.total === 0 ? (
          <div className="py-24 text-center">
            <Download size={36} className="mx-auto text-zinc-300 mb-3" />
            <p className="text-sm text-zinc-500">{t("stats.empty")}</p>
          </div>
        ) : stats && hero ? (
          <>
            {/* 大数字卡 */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <HeroCard
                icon={<Download size={16} />}
                label={t("stats.totalDownloads")}
                value={String(hero.success)}
                accent="text-blue-600"
              />
              <HeroCard
                icon={<HardDrive size={16} />}
                label={t("stats.totalSize")}
                value={formatSize(hero.total_size)}
                accent="text-emerald-600"
              />
              <HeroCard
                icon={<CheckCircle2 size={16} />}
                label={t("stats.successRate")}
                value={successRate(hero)}
                accent="text-indigo-600"
              />
              <HeroCard
                icon={<CalendarDays size={16} />}
                label={t("stats.usageDays")}
                value={String(usageDays)}
                accent="text-orange-600"
              />
            </div>

            {/* 次要指标行 */}
            <div className="mt-4 flex flex-wrap gap-x-8 gap-y-1 text-[13px] text-zinc-500">
              <span className="inline-flex items-center gap-1.5">
                <Clock size={13} />
                {t("stats.avgDuration", { d: formatDuration(hero.avg_duration) })}
              </span>
              <span>
                {t("stats.totalRecords", { n: hero.total })}
              </span>
              {hero.failed > 0 && (
                <span className="text-red-500">
                  {t("stats.failed", { n: hero.failed })}
                </span>
              )}
            </div>

            {/* 图表区 */}
            {!echartsReady ? (
              <div className="mt-8 py-10 text-center">
                <p className="text-sm text-zinc-400">{t("stats.chartLoading")}</p>
              </div>
            ) : (
              <div className="mt-8 space-y-6">
                <ChartCard title={t("stats.trendTitle")} subtitle={t("stats.trendSubtitle")}>
                  <TrendChart data={stats.daily} />
                </ChartCard>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <ChartCard title={t("stats.sourceTitle")} subtitle={t("stats.sourceSubtitle")}>
                    <PieChart data={stats.sources} />
                  </ChartCard>
                  <ChartCard title={t("stats.uploaderTitle")} subtitle={t("stats.uploaderSubtitle")}>
                    <BarChart data={stats.uploaders} />
                  </ChartCard>
                </div>

                <ChartCard title={t("stats.handleTitle")} subtitle={t("stats.handleSubtitle")}>
                  <RoseChart data={stats.handles.map((h) => ({ ...h, name: `@${h.name}` }))} />
                </ChartCard>
              </div>
            )}
          </>
        ) : null}
      </div>
    </div>
  );
}

/* ---------- 小部件 ---------- */

function HeroCard({
  icon,
  label,
  value,
  accent,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  accent: string;
}) {
  return (
    <div className="bg-white rounded-3xl px-5 py-4 shadow-[0_1px_3px_rgba(0,0,0,0.06)] border border-black/5">
      <div className={`flex items-center gap-1.5 text-[12px] font-medium ${accent} mb-1.5`}>
        {icon}
        {label}
      </div>
      <div className="text-[28px] font-semibold tracking-tight text-zinc-900 leading-none">
        {value}
      </div>
    </div>
  );
}

function ChartCard({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle: string;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-white rounded-3xl p-5 shadow-[0_1px_3px_rgba(0,0,0,0.06)] border border-black/5">
      <h3 className="text-[15px] font-semibold text-zinc-900">{title}</h3>
      <p className="text-[12px] text-zinc-500 mb-3">{subtitle}</p>
      {children}
    </div>
  );
}

/** ECharts 容器封装：init / setOption / resize / dispose。
 * 覆盖层首次挂载时容器可能尚未布局（宽度 0），用 ResizeObserver 在
 * 尺寸就绪/变化时 resize，确保图表在 Tauri/WebView2 里正常渲染。
 *
 * Tauri/WebView2 合成层就绪前首次 init 可能白屏（"点刷新后图表才出现"），
 * 这里在 init 后延迟校验：若容器无子元素或 chart 宽度为 0，dispose 后
 * 自动重新 init（模拟一次"刷新"）。最多重试 2 次。 */
function EChart({
  option,
  height = 260,
}: {
  option: any;
  height?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let disposed = false;
    let chart: any = null;
    let ro: ResizeObserver | null = null;
    let timers: number[] = [];

    const initChart = (echarts: any, attempt: number) => {
      if (disposed || !ref.current) return;
      const el = ref.current;
      try {
        chart = echarts.init(el, undefined, { renderer: "svg" });
        chart.setOption(option);
        chart.resize();
        setReady(true);
        // 延迟校验：WebView2 合成层未就绪时首次渲染可能白屏，
        // 若容器仍无子元素或 chart 宽度为 0，则 dispose 重试。
        const t1 = window.setTimeout(() => {
          if (disposed || !chart) return;
          const noChild = el.childElementCount === 0;
          const zeroW = chart.getWidth() === 0 || chart.getHeight() === 0;
          if ((noChild || zeroW) && attempt < 2) {
            chart.dispose();
            chart = null;
            initChart(echarts, attempt + 1);
          } else {
            // 即使看起来正常，也再 resize 一次兜底。
            chart.resize();
          }
        }, 300);
        timers.push(t1);
        if (typeof ResizeObserver !== "undefined") {
          ro = new ResizeObserver(() => chart?.resize());
          ro.observe(el);
        }
      } catch (e) {
        // init 失败静默（保持原有行为），ECharts 加载失败已有页面提示。
        console.error("[Stats] echarts init failed:", e);
      }
    };

    loadEcharts()
      .then((echarts) => {
        if (disposed || !ref.current) return;
        // 等一帧，确保布局完成、容器有真实尺寸再 init。
        requestAnimationFrame(() => initChart(echarts, 0));
      })
      .catch(() => {});
    const onResize = () => chart?.resize();
    window.addEventListener("resize", onResize);
    return () => {
      disposed = true;
      timers.forEach((t) => window.clearTimeout(t));
      ro?.disconnect();
      window.removeEventListener("resize", onResize);
      chart?.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [option]);

  return (
    <div
      ref={ref}
      style={{ height }}
      className={ready ? "w-full" : "w-full flex items-center justify-center text-zinc-300 text-xs"}
    >
      {!ready && "…"}
    </div>
  );
}

const BLUE = "#0071e3";
const BLUE_SOFT = "rgba(0,113,227,0.15)";
const PIE_COLORS = ["#0071e3", "#34c759", "#ff9500", "#af52de", "#ff3b30", "#5ac8fa", "#ffcc00", "#5856d6"];

/** 最近下载趋势：平滑折线 + 渐变面积。 */
function TrendChart({ data }: { data: { name: string; count: number }[] }) {
  const option = useMemo(
    () => ({
      grid: { left: 8, right: 16, top: 20, bottom: 8, containLabel: true },
      tooltip: { trigger: "axis" as const },
      xAxis: {
        type: "category" as const,
        data: data.map((d) => d.name),
        axisLine: { lineStyle: { color: "#e5e5e5" } },
        axisTick: { show: false },
        axisLabel: { color: "#999", fontSize: 11 },
      },
      yAxis: {
        type: "value" as const,
        minInterval: 1,
        splitLine: { lineStyle: { color: "#f0f0f0" } },
        axisLabel: { color: "#999", fontSize: 11 },
      },
      series: [
        {
          type: "line" as const,
          smooth: true,
          symbol: "none",
          data: data.map((d) => d.count),
          lineStyle: { color: BLUE, width: 2.5 },
          areaStyle: {
            color: {
              type: "linear" as const,
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: BLUE_SOFT },
                { offset: 1, color: "rgba(0,113,227,0)" },
              ],
            },
          },
        },
      ],
    }),
    [data]
  );
  return <EChart option={option} />;
}

/** 来源分布环形图。 */
function PieChart({ data }: { data: { name: string; count: number }[] }) {
  const option = useMemo(
    () => ({
      tooltip: { trigger: "item" as const },
      legend: {
        bottom: 0,
        icon: "circle" as const,
        itemWidth: 8,
        itemHeight: 8,
        textStyle: { color: "#666", fontSize: 11 },
      },
      series: [
        {
          type: "pie" as const,
          radius: ["52%", "72%"],
          center: ["50%", "42%"],
          avoidLabelOverlap: true,
          itemStyle: { borderRadius: 6, borderColor: "#fff", borderWidth: 2 },
          label: { show: false },
          emphasis: {
            label: { show: true, fontSize: 13, fontWeight: 600, formatter: "{b}\n{c}" },
          },
          data: data.map((d, i) => ({
            name: d.name,
            value: d.count,
            itemStyle: { color: PIE_COLORS[i % PIE_COLORS.length] },
          })),
        },
      ],
    }),
    [data]
  );
  return <EChart option={option} height={240} />;
}

/** Top 作者：横向柱状。 */
function BarChart({ data }: { data: { name: string; count: number }[] }) {
  const option = useMemo(
    () => ({
      grid: { left: 8, right: 24, top: 8, bottom: 8, containLabel: true },
      tooltip: { trigger: "axis" as const, axisPointer: { type: "shadow" as const } },
      xAxis: {
        type: "value" as const,
        minInterval: 1,
        splitLine: { lineStyle: { color: "#f0f0f0" } },
        axisLabel: { color: "#999", fontSize: 11 },
      },
      yAxis: {
        type: "category" as const,
        data: data.map((d) => d.name),
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: { color: "#555", fontSize: 11 },
      },
      series: [
        {
          type: "bar" as const,
          data: data.map((d) => d.count),
          barWidth: 14,
          itemStyle: {
            color: {
              type: "linear" as const,
              x: 0, y: 0, x2: 1, y2: 0,
              colorStops: [
                { offset: 0, color: BLUE },
                { offset: 1, color: "#5ac8fa" },
              ],
            },
            borderRadius: [0, 7, 7, 0],
          },
        },
      ],
    }),
    [data]
  );
  return <EChart option={option} height={240} />;
}

/** Top 账号：南丁格尔玫瑰图（与 Top 作者柱状图视觉区分）。 */
function RoseChart({ data }: { data: { name: string; count: number }[] }) {
  const option = useMemo(
    () => ({
      tooltip: { trigger: "item" as const, formatter: "{b}<br/>{c} 次" },
      legend: {
        bottom: 0,
        type: "scroll" as const,
        itemWidth: 8,
        itemHeight: 8,
        textStyle: { color: "#666", fontSize: 11 },
      },
      series: [
        {
          type: "pie" as const,
          roseType: "radius" as const,
          radius: ["18%", "82%"],
          center: ["50%", "44%"],
          itemStyle: { borderRadius: 6, borderColor: "#fff", borderWidth: 2 },
          label: {
            show: true,
            fontSize: 10,
            color: "#888",
            formatter: "{b}",
          },
          labelLine: { length: 8, length2: 6 },
          emphasis: {
            label: { show: true, fontSize: 13, fontWeight: 600, formatter: "{b}\n{c} 次" },
          },
          data: data.map((d, i) => ({
            name: d.name,
            value: d.count,
            itemStyle: { color: PIE_COLORS[i % PIE_COLORS.length] },
          })),
        },
      ],
    }),
    [data]
  );
  return <EChart option={option} height={260} />;
}

/* ---------- 格式化工具 ---------- */

function formatSize(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = bytes;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`;
}

function formatDuration(secs: number): string {
  const s = Math.round(secs);
  if (!s) return "0:00";
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
  return `${m}:${String(ss).padStart(2, "0")}`;
}

function successRate(hero: NonNullable<DownloadStats["hero"]>): string {
  const total = hero.success + hero.failed;
  if (!total) return "—";
  return `${((hero.success / total) * 100).toFixed(1)}%`;
}
