// 加载 ECharts：优先本地静态文件（离线可用），失败回退国内 CDN。
// 本地文件放 ui/public/vendor/echarts.min.js，构建后随 frontendDist 一起分发。
// 缓存 Promise：多次调用只注入一次 script；加载失败重置缓存允许重试。

const LOCAL_URL = "/vendor/echarts.min.js";

const CDN_URLS = [
  "https://cdn.bootcdn.net/ajax/libs/echarts/5.6.0/echarts.min.js",
  "https://cdn.staticfile.org/echarts/5.6.0/echarts.min.js",
  "https://cdn.jsdelivr.net/npm/echarts@5.6.0/dist/echarts.min.js",
];

let cached: Promise<any> | null = null;

/** 确保 ECharts 已加载，返回全局 echarts 对象。 */
export function loadEcharts(): Promise<any> {
  if (cached) return cached;
  if (window.echarts) {
    cached = Promise.resolve(window.echarts);
    return cached;
  }
  cached = loadFromUrls(0);
  return cached;
}

function loadFromUrls(index: number): Promise<any> {
  // index 0 = 本地文件；index >= 1 时从 CDN 列表取。
  const url = index === 0 ? LOCAL_URL : CDN_URLS[index - 1];
  if (index > CDN_URLS.length) {
    // 全部失败：重置缓存，下次调用重新尝试。
    cached = null;
    return Promise.reject(new Error("ECharts 加载失败，请检查网络"));
  }
  return new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = url;
    s.async = true;
    s.onload = () => {
      if (window.echarts) {
        resolve(window.echarts);
      } else {
        // 脚本加载了但全局对象未出现 → 尝试下一个源。
        loadFromUrls(index + 1).then(resolve, reject);
      }
    };
    s.onerror = () => {
      loadFromUrls(index + 1).then(resolve, reject);
    };
    document.head.appendChild(s);
  });
}
