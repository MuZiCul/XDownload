// 全局 echarts 声明（本地 vendor/echarts.min.js 优先，CDN 兜底；不安装 npm 实体包）。
// ECharts 由 echartsLoader.ts 动态注入 script，这里给出 window.echarts 的最小类型。
declare global {
  interface Window {
    echarts?: any;
  }
}

export {};
