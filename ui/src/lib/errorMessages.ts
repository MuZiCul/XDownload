/**
 * Convert raw error messages (from backend / yt-dlp) into friendly,
 * user-facing messages. Matched rules return an i18n key that is translated
 * through the active UI language; unmatched text is returned verbatim.
 *
 * The rules are matched against the raw yt-dlp error text (mostly from the
 * Twitter/X extractor). The FIRST matching rule wins, so order matters —
 * put the most specific rules first.
 */
import { t as i18nT } from "./i18n";

type Rule = { pattern: RegExp; key: string };

const RULES: Rule[] = [
  // User-initiated cancel (cancel button) — friendly message instead of the
  // raw stderr from the killed process.
  {
    pattern: /用户主动取消|user.?cancelled|主动取消/i,
    key: "error.cancelled",
  },
  // Account suspended (author banned by X).
  { pattern: /suspended/i, key: "error.suspended" },
  // Private / protected account — must be logged in and following.
  {
    pattern: /protected|not authorized|private account/i,
    key: "error.private",
  },
  // Tweet deleted / no longer available / tombstoned.
  {
    pattern: /tweet.*(?:unavailable|no longer|deleted)/i,
    key: "error.deleted",
  },
  // NSFW / age-restricted content.
  {
    pattern: /nsfw|age.?restricted|requires authentication/i,
    key: "error.nsfw",
  },
  // The tweet contains no downloadable video / the selected item is not a video.
  {
    pattern: /no video could be found|is not a video/i,
    key: "error.noVideo",
  },
  // Guest token / guest mode failure (X anti-bot measures).
  { pattern: /guest mode|guest token/i, key: "error.guestToken" },
  // Rate limiting.
  { pattern: /rate.?limit|http error 429/i, key: "error.rateLimit" },
  // Geo restriction.
  {
    pattern: /geoblocked|not available in your country/i,
    key: "error.geoblocked",
  },
  // Broadcast / live.
  { pattern: /broadcast no longer exists/i, key: "error.broadcast" },
  // Twitter Spaces.
  { pattern: /space not found|space.*ended/i, key: "error.space" },
  // Generic API error.
  { pattern: /error\(s\) while querying api/i, key: "error.api" },
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
      return i18nT(rule.key);
    }
  }
  return raw;
}
