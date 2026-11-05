/**
 * Internationalisation.
 *
 * Deliberately dependency-free: the string set is small, there is no plural
 * machinery beyond `Intl.PluralRules`, and a 9-language desktop app should not
 * pay for a runtime that also does lazy bundle splitting.
 *
 * Shape of a catalogue: a flat object of dotted keys. Flat keys (not nested
 * objects) make a missing translation a *compile* error, because the English
 * catalogue's type is derived and every other catalogue is checked against it.
 */

export const LOCALES = [
  "zh-CN",
  "zh-TW",
  "en",
  "es",
  "pt",
  "fr",
  "de",
  "ja",
  "ko",
] as const;

export type Locale = (typeof LOCALES)[number];

/** Native names, so the picker is readable to a speaker of that language. */
export const LOCALE_NAMES: Record<Locale, string> = {
  "zh-CN": "简体中文",
  "zh-TW": "繁體中文",
  en: "English",
  es: "Español",
  pt: "Português",
  fr: "Français",
  de: "Deutsch",
  ja: "日本語",
  ko: "한국어",
};

/** `Intl` locale tag for each app locale, used for dates and numbers. */
export const INTL_TAGS: Record<Locale, string> = {
  "zh-CN": "zh-CN",
  "zh-TW": "zh-TW",
  en: "en-US",
  es: "es-ES",
  pt: "pt-BR",
  fr: "fr-FR",
  de: "de-DE",
  ja: "ja-JP",
  ko: "ko-KR",
};

const STORAGE_KEY = "cube-studio.locale";

/**
 * Collapse an arbitrary BCP 47 tag onto a supported locale.
 *
 * Mirrors `normalise` in `src-tauri/src/locales.rs`; both exist because either
 * side may be asked (the OS for the initial value, the user for a choice).
 */
export function normaliseLocale(tag: string | null | undefined): Locale {
  if (!tag) return "en";
  const lower = tag.toLowerCase();
  const primary = lower.split(/[-_]/)[0] ?? "";

  if (primary === "zh") {
    const traditional =
      lower.includes("hant") ||
      lower.includes("-tw") ||
      lower.includes("-hk") ||
      lower.includes("-mo");
    return traditional ? "zh-TW" : "zh-CN";
  }
  if ((LOCALES as readonly string[]).includes(primary)) {
    return primary as Locale;
  }
  return "en";
}

/** Best guess from the environment, before the user has chosen. */
export function detectLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) return normaliseLocale(stored);
  } catch {
    // Private mode or a locked-down WebView: fall through to detection.
  }
  const navigatorLocale =
    typeof navigator !== "undefined"
      ? (navigator.languages?.[0] ?? navigator.language)
      : undefined;
  return normaliseLocale(navigatorLocale);
}

export function persistLocale(locale: Locale): void {
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // Persisting is best effort; the in-memory choice still applies.
  }
}

/**
 * Values interpolated into a message: `{count} keys` with `{ count: 5 }`.
 */
export type Vars = Record<string, string | number>;

/**
 * Replace `{name}` placeholders.
 *
 * An unknown placeholder is left verbatim rather than replaced with
 * `undefined`, so a typo in a translation is visible in the UI instead of
 * silently producing broken sentences.
 */
export function interpolate(template: string, vars?: Vars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
}

/** Plural category, for languages where the count changes the noun form. */
export function pluralForm(
  count: number,
  locale: Locale,
): Intl.LDMLPluralRule {
  try {
    return new Intl.PluralRules(INTL_TAGS[locale]).select(count);
  } catch {
    return count === 1 ? "one" : "other";
  }
}

/** Format a byte count for the given locale. */
export function formatNumber(
  value: number,
  locale: Locale,
  options?: Intl.NumberFormatOptions,
): string {
  try {
    return new Intl.NumberFormat(INTL_TAGS[locale], options).format(value);
  } catch {
    return String(value);
  }
}

/** Format a timestamp for the given locale. */
export function formatDateTime(
  iso: string | null | undefined,
  locale: Locale,
): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  try {
    return date.toLocaleString(INTL_TAGS[locale], {
      month: "short",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return date.toISOString();
  }
}
