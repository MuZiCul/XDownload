import { useSyncExternalStore } from "react";
import { emit } from "@tauri-apps/api/event";
import { getPrivacyMode as loadPersisted, setPrivacyModePersist } from "./bindings";

/**
 * 隐私模式全局状态（模块级 + useSyncExternalStore）。
 * 开启后任务页/下载页的标题以 *** 显示、封面毛玻璃覆盖。
 *
 * 状态持久化到后端 config/settings.json（privacy_mode 字段），重启后保持。
 * 状态变更时同时向后端 emit `privacy-mode-changed`，用于同步系统托盘菜单
 * 的「开启/退出隐私模式」文本。
 */
let privacyMode = false;
let initialized = false;
const listeners = new Set<() => void>();

function setState(next: boolean) {
  privacyMode = next;
  listeners.forEach((l) => l());
  emit("privacy-mode-changed", { enabled: next }).catch(() => {});
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function getSnapshot() {
  return privacyMode;
}

/** 应用启动时调用：从后端加载持久化的隐私模式状态。 */
export async function initPrivacyMode(): Promise<void> {
  if (initialized) return;
  initialized = true;
  try {
    privacyMode = await loadPersisted();
    listeners.forEach((l) => l());
  } catch (e) {
    console.warn("initPrivacyMode failed:", e);
  }
}

/** 读取隐私模式状态（订阅，组件内使用）。 */
export function usePrivacyMode(): boolean {
  return useSyncExternalStore(subscribe, getSnapshot);
}

/** 读取隐私模式当前值（非 hook，事件监听器等场景用）。 */
export function getPrivacyMode(): boolean {
  return privacyMode;
}

/** 切换隐私模式（持久化到后端配置，不阻塞 UI）。 */
export function setPrivacyMode(on: boolean) {
  setState(on);
  setPrivacyModePersist(on).catch((e) => console.warn("persist privacy failed:", e));
}
