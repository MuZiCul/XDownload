import { useSyncExternalStore } from "react";
import { loadSettings } from "../bindings";
import { zh } from "./zh";
import { en } from "./en";

export type Lang = "zh" | "en";

type Vars = Record<string, string | number | null | undefined>;

let lang: Lang = "zh";
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((l) => l());
}

/** Set the active UI language (re-renders every useI18n consumer). */
export function setLang(l: Lang) {
  if (lang === l) return;
  lang = l;
  emit();
}

/** Get the current language code. */
export function getLang(): Lang {
  return lang;
}

/**
 * Translate a key with optional `{var}` interpolation.
 * Falls back to the key itself when missing.
 */
export function t(key: string, vars?: Vars): string {
  const dict = lang === "en" ? en : zh;
  let str = dict[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      str = str.split(`{${k}}`).join(v == null ? "" : String(v));
    }
  }
  return str;
}

/** React hook: subscribe to the current language + translation function. */
export function useI18n(): { lang: Lang; t: typeof t } {
  useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => lang
  );
  return { lang, t };
}

let initialized = false;

/** Load the persisted language (from settings) once. Call from App on mount. */
export async function initI18n() {
  if (initialized) return;
  initialized = true;
  try {
    const s = await loadSettings();
    lang = s.lang === "en" ? "en" : "zh";
    emit();
  } catch {
    // Keep the default (zh).
  }
}
