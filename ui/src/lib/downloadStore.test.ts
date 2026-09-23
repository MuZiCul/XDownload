import { describe, expect, it } from "vitest";
import {
  resolveSelection,
  shallowArrayEqual,
  type DownloadState,
} from "./downloadStore";

/** 构造一个只带 id/url 的队列状态（测试只关心 URL 切片与选择器缓存）。 */
function stateWithUrls(urls: string[]): DownloadState {
  return {
    queueTasks: urls.map((url, i) => ({ id: String(i), url })) as unknown as DownloadState["queueTasks"],
  };
}

describe("resolveSelection（选择器缓存：useSyncExternalStore 不无限重渲染的关键）", () => {
  it("无选择器时原样返回整个 state（引用稳定）", () => {
    const s = stateWithUrls(["a"]);
    const a = resolveSelection(null, s, undefined, Object.is);
    const b = resolveSelection(a, s, undefined, Object.is);
    expect(a.out).toBe(s);
    expect(b.out).toBe(s);
  });

  it("快照引用未变时复用上一条缓存（不重复计算选择器）", () => {
    const s = stateWithUrls(["a"]);
    let calls = 0;
    const selector = (st: DownloadState) => {
      calls += 1;
      return st.queueTasks.length;
    };
    const first = resolveSelection(null, s, selector, Object.is);
    const second = resolveSelection(first, s, selector, Object.is);
    expect(second).toBe(first);
    expect(calls).toBe(1);
  });

  it("选择结果等价时复用旧引用（避免快照持续变化导致无限重渲染）", () => {
    // 模拟两次进度 tick：queueTasks 是新数组，但 URL 切片内容相同。
    const s1 = stateWithUrls(["a", "b"]);
    const s2 = stateWithUrls(["a", "b"]);
    expect(s1).not.toBe(s2);

    const selector = (st: DownloadState) => st.queueTasks.map((t) => t.url ?? "");
    const first = resolveSelection(null, s1, selector, shallowArrayEqual);
    const second = resolveSelection(first, s2, selector, shallowArrayEqual);

    // 关键断言：内容相同 → 必须复用第一个结果的引用。
    expect(second.out).toBe(first.out);
    expect(second.snap).toBe(s2);
  });

  it("选择结果真正变化时返回新值", () => {
    const s1 = stateWithUrls(["a"]);
    const s2 = stateWithUrls(["a", "b"]);
    const selector = (st: DownloadState) => st.queueTasks.map((t) => t.url ?? "");
    const first = resolveSelection(null, s1, selector, shallowArrayEqual);
    const second = resolveSelection(first, s2, selector, shallowArrayEqual);
    expect(second.out).toEqual(["a", "b"]);
    expect(second.out).not.toBe(first.out);
  });

  it("布尔切片：进度 tick 不改变选择结果时保持同一引用", () => {
    const selector = (st: DownloadState) =>
      st.queueTasks.some((t) => t.status === "downloading");
    const first = resolveSelection(null, stateWithUrls(["a"]), selector, Object.is);
    const second = resolveSelection(
      first,
      stateWithUrls(["a"]),
      selector,
      Object.is
    );
    expect(second.out).toBe(first.out);
  });
});

describe("shallowArrayEqual", () => {
  it("长度或元素不同 → false；完全相同 → true", () => {
    expect(shallowArrayEqual(["a"], ["a"])).toBe(true);
    expect(shallowArrayEqual(["a"], ["a", "b"])).toBe(false);
    expect(shallowArrayEqual(["a"], ["b"])).toBe(false);
    expect(shallowArrayEqual([], [])).toBe(true);
    // 同一引用直接 true。
    const arr = ["a"];
    expect(shallowArrayEqual(arr, arr)).toBe(true);
  });
});
