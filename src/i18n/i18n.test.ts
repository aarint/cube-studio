import { describe, expect, it } from "vitest";

import {
  LOCALES,
  detectLocale,
  interpolate,
  normaliseLocale,
  pluralForm,
} from "@/i18n";
import { CATALOGUES } from "@/i18n/I18nProvider";
import { en, type MessageKey } from "@/i18n/en";

const KEYS = Object.keys(en) as MessageKey[];

/** Placeholder names in a template, e.g. `{count}` -> "count". */
function placeholders(template: string): string[] {
  return [...template.matchAll(/\{(\w+)\}/g)].map((match) => match[1]!);
}

describe("catalogues", () => {
  it("covers every locale", () => {
    expect(Object.keys(CATALOGUES).sort()).toEqual([...LOCALES].sort());
  });

  it.each(LOCALES)("%s has every key", (locale) => {
    const catalogue = CATALOGUES[locale];
    const missing = KEYS.filter((key) => !catalogue[key]);
    expect(missing, `${locale} is missing keys`).toEqual([]);
  });

  it.each(LOCALES)("%s has no keys English does not", (locale) => {
    // A key that exists only in a translation is dead weight and usually a typo.
    const extra = Object.keys(CATALOGUES[locale]).filter(
      (key) => !(key in en),
    );
    expect(extra, `${locale} has unknown keys`).toEqual([]);
  });

  it.each(LOCALES)("%s keeps every placeholder", (locale) => {
    // Losing a {placeholder} silently drops data from the UI, and adding one
    // that English lacks means the value is never supplied.
    const broken: string[] = [];
    for (const key of KEYS) {
      const source = placeholders(en[key]).sort();
      const target = placeholders(CATALOGUES[locale][key]).sort();
      if (source.join() !== target.join()) {
        broken.push(`${key}: expected {${source.join("},{")}} got {${target.join("},{")}}`);
      }
    }
    expect(broken, `${locale} placeholder mismatches`).toEqual([]);
  });

  it.each(LOCALES)("%s has no untranslated English left in non-English locales", (locale) => {
    if (locale === "en") return;
    // Some strings are legitimately identical (product names, "TLS", "JSON",
    // bare commands like `GET some:key`, and words that happen to share a
    // spelling). Flag only prose that was copied verbatim, which is the usual
    // sign of a forgotten key.
    const allowedIdentical = new Set<MessageKey>([
      "app.name",
      "dialog.redis",
      "dialog.mqtt",
      "dialog.tlsMode",
      "detail.json",
      "workspace.console",
      "console.placeholder.redis",
      "console.placeholder.memcached",
      "console.placeholder.rabbit",
      "publish.qos",
      // French "session"/"sessions" is spelled exactly as in English.
      "rail.sessions",
      "rail.sessions_plural",
      "tail.binary",
    ]);

    const suspicious = KEYS.filter((key) => {
      if (allowedIdentical.has(key)) return false;
      const value = CATALOGUES[locale][key];
      // Only consider strings long enough to be prose rather than a token.
      return value === en[key] && en[key].length > 12;
    });

    expect(suspicious, `${locale} still has English text`).toEqual([]);
  });

  it("every catalogue is non-trivial", () => {
    for (const locale of LOCALES) {
      const values = Object.values(CATALOGUES[locale]);
      expect(values.length).toBe(KEYS.length);
      // A catalogue of empty strings would pass the key check but render blank.
      expect(values.every((value) => value.trim().length > 0)).toBe(true);
    }
  });
});

describe("interpolate", () => {
  it("substitutes values", () => {
    expect(interpolate("Hello {name}", { name: "Ada" })).toBe("Hello Ada");
  });

  it("leaves unknown placeholders visible", () => {
    // Better to show `{count}` in the UI than the word "undefined".
    expect(interpolate("{count} items", {})).toBe("{count} items");
  });

  it("handles numbers", () => {
    expect(interpolate("{count} sessions", { count: 3 })).toBe("3 sessions");
  });

  it("passes text through when no vars are given", () => {
    expect(interpolate("plain")).toBe("plain");
  });
});

describe("normaliseLocale", () => {
  it.each([
    ["en", "en"],
    ["en-GB", "en"],
    ["en_US", "en"],
    ["pt-BR", "pt"],
    ["fr-CA", "fr"],
    ["de-AT", "de"],
    ["ja-JP", "ja"],
    ["ko-KR", "ko"],
    ["es-419", "es"],
  ])("maps %s to %s", (input, expected) => {
    expect(normaliseLocale(input)).toBe(expected);
  });

  it("separates Chinese scripts", () => {
    expect(normaliseLocale("zh-CN")).toBe("zh-CN");
    expect(normaliseLocale("zh-Hans")).toBe("zh-CN");
    expect(normaliseLocale("zh")).toBe("zh-CN");
    expect(normaliseLocale("zh-TW")).toBe("zh-TW");
    expect(normaliseLocale("zh-Hant")).toBe("zh-TW");
    expect(normaliseLocale("zh-Hant-HK")).toBe("zh-TW");
    expect(normaliseLocale("zh-MO")).toBe("zh-TW");
  });

  it("falls back to English", () => {
    expect(normaliseLocale("ru")).toBe("en");
    expect(normaliseLocale("")).toBe("en");
    expect(normaliseLocale(null)).toBe("en");
    expect(normaliseLocale(undefined)).toBe("en");
  });
});

describe("detectLocale", () => {
  it("returns a supported locale", () => {
    expect(LOCALES).toContain(detectLocale());
  });
});

describe("pluralForm", () => {
  it("distinguishes one from other in English", () => {
    expect(pluralForm(1, "en")).toBe("one");
    expect(pluralForm(0, "en")).toBe("other");
    expect(pluralForm(2, "en")).toBe("other");
  });

  it("treats one and other the same in Chinese and Japanese", () => {
    // These languages have no grammatical plural, so both counts take `other`.
    expect(pluralForm(1, "zh-CN")).toBe("other");
    expect(pluralForm(2, "zh-CN")).toBe("other");
    expect(pluralForm(1, "ja")).toBe("other");
    expect(pluralForm(1, "ko")).toBe("other");
  });

  it("handles zero specially in French", () => {
    // French uses the singular form for 0 and 1.
    expect(pluralForm(0, "fr")).toBe("one");
    expect(pluralForm(1, "fr")).toBe("one");
    expect(pluralForm(2, "fr")).toBe("other");
  });
});
