import { describe, expect, it } from "vitest";
import {
  formatDateTime,
  formatDuration,
  formatFileSize,
  formatNumber,
} from "./format";

const t = (key: string) => key;

describe("formatDuration", () => {
  it("格式化时分秒", () => {
    expect(formatDuration(65)).toBe("1:05");
    expect(formatDuration(3600)).toBe("1:00:00");
    expect(formatDuration(3661)).toBe("1:01:01");
    expect(formatDuration(59)).toBe("0:59");
  });

  it("补零", () => {
    expect(formatDuration(61)).toBe("1:01");
    expect(formatDuration(3605)).toBe("1:00:05");
  });

  it("非法输入返回 ?", () => {
    expect(formatDuration(0)).toBe("?");
    expect(formatDuration(-5)).toBe("?");
    expect(formatDuration(undefined as unknown as number)).toBe("?");
  });
});

describe("formatNumber", () => {
  it("大数缩写", () => {
    expect(formatNumber(150000000, t)).toBe("1.5num.billion");
    expect(formatNumber(12345, t)).toBe("1.2num.tenThousand");
    expect(formatNumber(9999, t)).toBe("9,999");
  });

  it("普通数字", () => {
    expect(formatNumber(0, t)).toBe("0");
    expect(formatNumber(123, t)).toBe("123");
    expect(formatNumber(10000000, t)).toBe("1000.0num.tenThousand");
  });
});

describe("formatFileSize", () => {
  it("单位换算", () => {
    expect(formatFileSize(500)).toBe("500 B");
    expect(formatFileSize(1024)).toBe("1.0 KB");
    expect(formatFileSize(1536)).toBe("1.5 KB");
    expect(formatFileSize(1024 * 1024)).toBe("1.0 MB");
    expect(formatFileSize(1024 * 1024 * 1024)).toBe("1.0 GB");
  });

  it("大数字取整", () => {
    expect(formatFileSize(102400)).toBe("100 KB");
  });

  it("非法输入返回空串", () => {
    expect(formatFileSize(null)).toBe("");
    expect(formatFileSize(undefined)).toBe("");
    expect(formatFileSize(0)).toBe("");
    expect(formatFileSize(-1)).toBe("");
  });
});

describe("formatDateTime", () => {
  it("格式化时间戳", () => {
    // 2021-01-02 03:04:05 UTC
    expect(formatDateTime(1609556645)).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
  });

  it("零值返回 —", () => {
    expect(formatDateTime(0)).toBe("—");
    expect(formatDateTime(undefined as unknown as number)).toBe("—");
  });
});
