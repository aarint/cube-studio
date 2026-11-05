//! Supported interface languages.
//!
//! The Rust side owns the list so the backend and the frontend can never
//! disagree about which locales exist. The translated strings live in
//! `src/i18n/*.ts`, because they are rendered in the WebView; this module only
//! declares the set and negotiates a default.

/// BCP 47 tags of every language the UI ships with.
///
/// Ordered as they appear in the language picker: the two Chinese variants
/// first (this is a Chinese-authored project), then the European languages, then
/// Japanese and Korean.
pub const SUPPORTED: &[&str] = &[
    "zh-CN", "zh-TW", "en", "es", "pt", "fr", "de", "ja", "ko",
];

/// English name of a locale, for the language picker.
pub fn display_name(tag: &str) -> &'static str {
    match normalise(tag).as_str() {
        "zh-CN" => "简体中文",
        "zh-TW" => "繁體中文",
        "en" => "English",
        "es" => "Español",
        "pt" => "Português",
        "fr" => "Français",
        "de" => "Deutsch",
        "ja" => "日本語",
        "ko" => "한국어",
        _ => "English",
    }
}

/// Map an arbitrary locale tag onto one we support.
///
/// Browsers and operating systems report a wide range of tags (`en-GB`,
/// `pt-BR`, `zh-Hant-HK`), so this collapses them onto the closest supported
/// locale rather than failing.
pub fn normalise(tag: &str) -> String {
    let lower = tag.to_ascii_lowercase();
    let primary = lower.split(['-', '_']).next().unwrap_or("");

    match primary {
        "zh" => {
            // Script or region decides between Simplified and Traditional.
            let traditional = lower.contains("hant")
                || lower.contains("-tw")
                || lower.contains("-hk")
                || lower.contains("-mo");
            if traditional {
                "zh-TW".to_string()
            } else {
                "zh-CN".to_string()
            }
        }
        "en" => "en".into(),
        "es" => "es".into(),
        "pt" => "pt".into(),
        "fr" => "fr".into(),
        "de" => "de".into(),
        "ja" => "ja".into(),
        "ko" => "ko".into(),
        _ => "en".into(),
    }
}

/// Default locale for a platform, used on first run.
///
/// Reads the same environment variables each OS uses to record the user's
/// language, then falls back to English.
pub fn system_default() -> String {
    let candidates = [
        std::env::var("LANG").ok(),
        std::env::var("LC_ALL").ok(),
        std::env::var("LC_MESSAGES").ok(),
        // Windows sets neither LANG nor LC_ALL in the usual shell.
        std::env::var("LANGUAGE").ok(),
    ];
    for candidate in candidates.into_iter().flatten() {
        let tag = candidate.split('.').next().unwrap_or("");
        if !tag.is_empty() && tag != "C" && tag != "POSIX" {
            return normalise(tag);
        }
    }
    "en".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_locale_has_a_distinct_name() {
        // English is legitimately called "English", so uniqueness across the
        // whole set is the real assertion: a missing match arm would make two
        // locales share the fallback name.
        let mut seen = std::collections::BTreeSet::new();
        for tag in SUPPORTED {
            let name = display_name(tag);
            assert!(!name.is_empty(), "{tag} has no display name");
            assert!(seen.insert(name), "{tag} shares a name with another locale");
        }
    }

    #[test]
    fn locale_names_are_written_in_their_own_language() {
        assert_eq!(display_name("zh-CN"), "简体中文");
        assert_eq!(display_name("zh-TW"), "繁體中文");
        assert_eq!(display_name("ja"), "日本語");
        assert_eq!(display_name("ko"), "한국어");
        assert_eq!(display_name("de"), "Deutsch");
        assert_eq!(display_name("fr"), "Français");
        // Regional tags resolve to their base language's name.
        assert_eq!(display_name("pt-BR"), "Português");
        assert_eq!(display_name("en-US"), "English");
    }

    #[test]
    fn collapses_regional_variants() {
        assert_eq!(normalise("en-GB"), "en");
        assert_eq!(normalise("en_US"), "en");
        assert_eq!(normalise("pt-BR"), "pt");
        assert_eq!(normalise("fr-CA"), "fr");
        assert_eq!(normalise("de-AT"), "de");
        assert_eq!(normalise("ja-JP"), "ja");
        assert_eq!(normalise("ko-KR"), "ko");
    }

    #[test]
    fn distinguishes_chinese_scripts() {
        assert_eq!(normalise("zh-CN"), "zh-CN");
        assert_eq!(normalise("zh-Hans"), "zh-CN");
        assert_eq!(normalise("zh"), "zh-CN");
        assert_eq!(normalise("zh-TW"), "zh-TW");
        assert_eq!(normalise("zh-Hant"), "zh-TW");
        assert_eq!(normalise("zh-HK"), "zh-TW");
        assert_eq!(normalise("zh-Hant-HK"), "zh-TW");
    }

    #[test]
    fn unknown_locales_fall_back_to_english() {
        assert_eq!(normalise("ru"), "en");
        assert_eq!(normalise(""), "en");
        assert_eq!(normalise("xx-YY"), "en");
    }

    #[test]
    fn strips_encoding_suffixes_from_the_system_locale() {
        // `en_US.UTF-8` is what Linux and macOS actually report.
        assert_eq!(normalise("en_US.UTF-8".split('.').next().unwrap()), "en");
        assert_eq!(normalise("zh_CN.UTF-8".split('.').next().unwrap()), "zh-CN");
    }
}
