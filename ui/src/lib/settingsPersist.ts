import { loadSettings, saveSettings } from "./bindings";
import type { AppSettings } from "./types";

/**
 * 全局串行「读-改-写」保存链。
 *
 * 各设置卡片（Dir/Proxy/MultiTask/Hls）都会 loadSettings() → 改自己字段 →
 * saveSettings() 全量写盘。若并发执行，后保存的快照会把先前卡片的修改覆盖回旧值
 * （跨卡片竞态）。这里把整个「读-改-写」流程串行化：同一时刻只有一个 mutation
 * 在跑，从根本上避免互相覆盖。
 *
 * 用法：
 *   mutateAndSaveSettings((cfg) => { cfg.concurrency = 2; });
 * 返回 Promise，可 await 或忽略（队列内部已 catch，不影响 UI）。
 */
let chain: Promise<void> = Promise.resolve();

export function mutateAndSaveSettings(
  mutate: (cfg: AppSettings) => void
): Promise<void> {
  const task = chain.then(async () => {
    const cfg = await loadSettings();
    mutate(cfg);
    await saveSettings(cfg);
  });
  // 失败不阻断后续保存；错误由调用方通过 try/catch 捕获后提示。
  chain = task.catch(() => {});
  return task;
}
