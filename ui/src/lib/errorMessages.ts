/**
 * Convert raw error messages (from backend / yt-dlp) into friendly,
 * user-facing Chinese messages.
 *
 * The rules are matched against the raw yt-dlp error text (mostly from the
 * Twitter/X extractor). The FIRST matching rule wins, so order matters —
 * put the most specific rules first.
 */

type Rule = { pattern: RegExp; message: string };

const RULES: Rule[] = [
  // Account suspended (author banned by X).
  { pattern: /suspended/i, message: "该视频作者已被 X 封禁，无法获取视频内容" },
  // Private / protected account — must be logged in and following.
  {
    pattern: /protected|not authorized|private account/i,
    message: "该账号为私密/受保护账号，需登录并关注后才能查看",
  },
  // Tweet deleted / no longer available / tombstoned.
  {
    pattern: /tweet.*(?:unavailable|no longer|deleted)/i,
    message: "该推文已被删除或不可用",
  },
  // NSFW / age-restricted content.
  {
    pattern: /nsfw|age.?restricted|requires authentication/i,
    message: "该内容需要登录或年龄验证后才能查看",
  },
  // The tweet contains no downloadable video / the selected item is not a video.
  {
    pattern: /no video could be found|is not a video/i,
    message: "该推文中没有可下载的视频",
  },
  // Guest token / guest mode failure (X anti-bot measures).
  {
    pattern: /guest mode|guest token/i,
    message: "获取访客身份失败，请尝试设置 Cookies 或代理后重试",
  },
  // Rate limiting.
  {
    pattern: /rate.?limit|http error 429/i,
    message: "请求过于频繁，请稍后重试",
  },
  // Geo restriction.
  {
    pattern: /geoblocked|not available in your country/i,
    message: "该内容在您所在地区不可用",
  },
  // Broadcast / live.
  { pattern: /broadcast no longer exists/i, message: "该直播已结束或不存在" },
  // Twitter Spaces.
  {
    pattern: /space not found|space.*ended/i,
    message: "该 Space 不存在或已结束",
  },
  // Generic API error.
  {
    pattern: /error\(s\) while querying api/i,
    message: "X 接口返回异常，请稍后重试",
  },
];

export function friendlyErrorMessage(err: unknown): string {
  const raw =
    typeof err === "string"
      ? err
      : err instanceof Error
        ? err.message
        : `${err}`;
  for (const rule of RULES) {
    if (rule.pattern.test(raw)) {
      return rule.message;
    }
  }
  return raw;
}
