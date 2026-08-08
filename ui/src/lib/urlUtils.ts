/**
 * URL 校验与提取工具 —— 与后端 `is_supported_url`（commands/download.rs）
 * 的域名白名单保持一致，避免前后端判定差异。
 */

/** 是否为一个受支持的 X/Twitter 链接（x.com / twitter.com 及其子域）。 */
export function isSupportedUrl(raw: string): boolean {
  let u: URL;
  try {
    u = new URL(raw.trim());
  } catch {
    return false;
  }
  if (u.protocol !== "http:" && u.protocol !== "https:") return false;
  const host = u.hostname.toLowerCase();
  return (
    host === "x.com" ||
    host.endsWith(".x.com") ||
    host === "twitter.com" ||
    host.endsWith(".twitter.com")
  );
}

/** 常见中英文尾随标点。 */
const TRAILING_PUNCT = /[,.;:!?"')\]}>，。；：！？"’）》】]+$/;

/**
 * 按 `http(s)://` 协议头把文本切成若干段。每个协议头是新的链接起点，
 * 段尾延伸到下一个协议头之前 —— 这样链接连在一起（无空白）也能分开：
 *   "https://x.com/ahttps://twitter.com/b" → 切成两个段。
 */
function splitByProtocol(text: string): string[] {
  const re = /https?:\/\//gi;
  const starts: number[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    starts.push(m.index);
  }
  const segs: string[] = [];
  for (let i = 0; i < starts.length; i++) {
    const begin = starts[i];
    const end = i + 1 < starts.length ? starts[i + 1] : text.length;
    segs.push(text.slice(begin, end));
  }
  return segs;
}

/**
 * 从任意文本中提取所有受支持的链接：
 * - 按协议头重叠切段（处理链接连在一起）
 * - 每段取第一个空白前的部分（段内可能夹杂文字），去尾随中英文标点
 * - 去重
 */
export function extractLinks(text: string): string[] {
  const seen = new Set<string>();
  const links: string[] = [];
  for (const seg of splitByProtocol(text)) {
    // 优先取第一个空白前的部分（正常分隔 / 夹文字场景）。
    let candidate = seg.split(/\s+/)[0].replace(TRAILING_PUNCT, "");
    if (!isSupportedUrl(candidate)) {
      // 无空白（链接连在一起时该段即完整链接），尝试整段。
      candidate = seg.replace(TRAILING_PUNCT, "");
    }
    if (isSupportedUrl(candidate) && !seen.has(candidate)) {
      seen.add(candidate);
      links.push(candidate);
    }
  }
  return links;
}
