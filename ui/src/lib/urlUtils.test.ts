import { describe, expect, it } from "vitest";
import { extractLinks, isSupportedUrl } from "./urlUtils";

describe("isSupportedUrl", () => {
  it("接受 x.com / twitter.com 及其子域", () => {
    expect(isSupportedUrl("https://x.com/user/status/123")).toBe(true);
    expect(isSupportedUrl("https://twitter.com/user/status/123")).toBe(true);
    expect(isSupportedUrl("http://x.com/a")).toBe(true);
    expect(isSupportedUrl("https://www.x.com/a")).toBe(true);
    expect(isSupportedUrl("https://mobile.twitter.com/a")).toBe(true);
    expect(isSupportedUrl("https://x.com/status/1?v=2#frag")).toBe(true);
    expect(isSupportedUrl("  https://x.com/a  ")).toBe(true);
  });

  it("拒绝其他域名与非法输入", () => {
    expect(isSupportedUrl("https://evilx.com/a")).toBe(false);
    expect(isSupportedUrl("https://x.com.evil.com/a")).toBe(false);
    expect(isSupportedUrl("https://notx.com/a")).toBe(false);
    expect(isSupportedUrl("https://example.com/")).toBe(false);
    expect(isSupportedUrl("")).toBe(false);
    expect(isSupportedUrl("not a url")).toBe(false);
    expect(isSupportedUrl("ftp://x.com/a")).toBe(false);
  });
});

describe("extractLinks", () => {
  it("空白分隔的多个链接", () => {
    const links = extractLinks(
      "https://x.com/a/status/1 https://twitter.com/b/status/2"
    );
    expect(links).toEqual([
      "https://x.com/a/status/1",
      "https://twitter.com/b/status/2",
    ]);
  });

  it("链接连在一起（无空白）也能分开", () => {
    const links = extractLinks(
      "https://x.com/a/status/1https://twitter.com/b/status/2"
    );
    expect(links).toEqual([
      "https://x.com/a/status/1",
      "https://twitter.com/b/status/2",
    ]);
  });

  it("夹文字的链接只取链接部分", () => {
    const links = extractLinks("看看这个 https://x.com/a/status/1 不错");
    expect(links).toEqual(["https://x.com/a/status/1"]);
  });

  it("去除尾随中英文标点", () => {
    const links = extractLinks("https://x.com/a/status/1，https://twitter.com/b/status/2。");
    expect(links).toEqual([
      "https://x.com/a/status/1",
      "https://twitter.com/b/status/2",
    ]);
  });

  it("去重", () => {
    const links = extractLinks(
      "https://x.com/a/status/1 https://x.com/a/status/1"
    );
    expect(links).toEqual(["https://x.com/a/status/1"]);
  });

  it("无链接返回空数组", () => {
    expect(extractLinks("hello world")).toEqual([]);
    expect(extractLinks("")).toEqual([]);
  });

  it("忽略不支持域名的链接", () => {
    const links = extractLinks(
      "https://youtube.com/watch?v=x https://x.com/a/status/1"
    );
    expect(links).toEqual(["https://x.com/a/status/1"]);
  });
});
