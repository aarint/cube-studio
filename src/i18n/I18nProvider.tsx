/**
 * React binding for the catalogues.
 *
 * Split from `index.ts` so the pure helpers (locale normalisation, interpolation)
 * stay importable from tests and from non-React code without pulling in React.
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import {
  LOCALES,
  LOCALE_NAMES,
  detectLocale,
  interpolate,
  normaliseLocale,
  persistLocale,
  pluralForm,
  type Locale,
  type Vars,
} from "./index";
import { en, type Catalogue, type MessageKey } from "./en";
import { de } from "./de";
import { es } from "./es";
import { fr } from "./fr";
import { ja } from "./ja";
import { ko } from "./ko";
import { pt } from "./pt";
import { zhCN } from "./zh-CN";
import { zhTW } from "./zh-TW";

/**
 * Every catalogue, keyed by locale.
 *
 * Listed eagerly rather than lazily imported: the nine catalogues total roughly
 * 100 KB of text, which is less than one round trip's latency would cost, and it
 * keeps `t()` synchronous — important, because it is called during render.
 */
export const CATALOGUES: Record<Locale, Catalogue> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en,
  es,
  pt,
  fr,
  de,
  ja,
  ko,
};

export interface I18nValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Translate a key, interpolating `vars`. */
  t: (key: MessageKey, vars?: Vars) => string;
  /**
   * Translate a pluralised key.
   *
   * Looks for `key_plural` when the locale's plural rule is not "one", so a
   * catalogue only needs the second form where the language actually has one.
   */
  tn: (key: MessageKey, count: number, vars?: Vars) => string;
  /** False when this string is showing English because the locale lacks it. */
  hasTranslation: (key: MessageKey) => boolean;
}

const I18nContext = createContext<I18nValue | null>(null);

/**
 * Look a key up, falling back to English.
 *
 * TypeScript guarantees every catalogue is complete, so the fallback only
 * triggers for a catalogue that was shipped stale — the runtime check keeps that
 * from rendering `undefined`.
 */
function lookup(locale: Locale, key: MessageKey): string {
  const catalogue = CATALOGUES[locale];
  const value = catalogue?.[key];
  if (typeof value === "string" && value.length > 0) {
    return value;
  }
  return en[key];
}

export function I18nProvider({
  children,
  initialLocale,
}: {
  children: ReactNode;
  /** Overrides detection; used by tests and by the preview harness. */
  initialLocale?: Locale;
}) {
  const [locale, setLocaleState] = useState<Locale>(
    () => initialLocale ?? detectLocale(),
  );

  // Keep <html lang> in step so the WebView applies correct hyphenation, font
  // fallback (notably for Japanese and Korean) and screen-reader pronunciation.
  useEffect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = locale;
    }
  }, [locale]);

  const setLocale = useCallback((next: Locale) => {
    const resolved = normaliseLocale(next);
    setLocaleState(resolved);
    persistLocale(resolved);
  }, []);

  const value = useMemo<I18nValue>(() => {
    const t = (key: MessageKey, vars?: Vars): string =>
      interpolate(lookup(locale, key), vars);

    const tn = (key: MessageKey, count: number, vars?: Vars): string => {
      // Pluralised catalogues use `key_plural` for the non-singular form.
      const pluralKey = `${key}_plural` as MessageKey;
      const usePlural = pluralForm(count, locale) !== "one";
      const chosen =
        usePlural && pluralKey in CATALOGUES[locale] ? pluralKey : key;
      return interpolate(lookup(locale, chosen), { count, ...vars });
    };

    const hasTranslation = (key: MessageKey): boolean =>
      CATALOGUES[locale]?.[key] !== undefined;

    return { locale, setLocale, t, tn, hasTranslation };
  }, [locale, setLocale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

/** Access the translator. Throws when used outside the provider. */
export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return value;
}

export { LOCALES, LOCALE_NAMES };
export type { Locale, MessageKey };
