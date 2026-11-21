import { useEffect, useRef, useState } from "react";

import { IconClose } from "@/components/icons";
import { LOCALES, LOCALE_NAMES, useI18n, type Locale } from "@/i18n/I18nProvider";

/**
 * Language picker.
 *
 * A button that opens a popover rather than a native `<select>`: each option is
 * rendered in its own language and script, which a `<select>` cannot do reliably
 * across platforms (Windows and Linux substitute fonts per-option
 * inconsistently, and some render CJK options as boxes).
 */
export function LanguagePicker() {
  const { locale, setLocale, t } = useI18n();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click and on Escape, the two things people try first.
  useEffect(() => {
    if (!open) return undefined;

    const onPointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };

    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const choose = (next: Locale) => {
    setLocale(next);
    setOpen(false);
  };

  return (
    <div className="lang" ref={containerRef}>
      <button
        type="button"
        className="lang__trigger"
        onClick={() => setOpen((value) => !value)}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={t("language.choose")}
        title={t("language.choose")}
      >
        <span className="lang__globe" aria-hidden="true">
          {/* A circle with meridians: reads as "language" at 14px without an
              icon-font dependency. */}
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4">
            <circle cx="8" cy="8" r="6" />
            <ellipse cx="8" cy="8" rx="2.6" ry="6" />
            <path d="M2 8h12" />
          </svg>
        </span>
        <span className="lang__current">{LOCALE_NAMES[locale]}</span>
      </button>

      {open ? (
        <div className="lang__menu" role="listbox" aria-label={t("language.choose")}>
          <div className="lang__menu-header">
            <span>{t("language.label")}</span>
            <button
              type="button"
              className="lang__menu-close"
              onClick={() => setOpen(false)}
              aria-label={t("toast.dismiss")}
            >
              <IconClose size={11} />
            </button>
          </div>
          {LOCALES.map((candidate) => (
            <button
              key={candidate}
              type="button"
              role="option"
              aria-selected={candidate === locale}
              className={`lang__option ${
                candidate === locale ? "lang__option--active" : ""
              }`}
              onClick={() => choose(candidate)}
            >
              <span className="lang__option-name">{LOCALE_NAMES[candidate]}</span>
              <span className="lang__option-tag">{candidate}</span>
              {candidate === locale ? (
                <span className="lang__option-check" aria-hidden="true">
                  ✓
                </span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
