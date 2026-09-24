//! Port of `java.util.Locale`, limited to what the message source needs.
//!
//! A locale is the combination of a language and, optionally, a country and a
//! variant, exactly like the JDK models it. The values are used to derive the
//! resource bundle suffixes of a basename, so `zh_CN` resolves to
//! `messages_zh_CN.properties` and falls back to `messages_zh.properties`.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// A language, optionally combined with a country and a variant.
///
/// The language is stored in lower case and the country in upper case, so the
/// value is normalized no matter how it was constructed. The root locale has no
/// language, no country and no variant; it maps to the locale independent
/// bundle, `messages.properties` for the basename `messages`.
///
/// # Examples
///
/// ```rust
/// use next_web_context::Locale;
///
/// let locale = Locale::for_language_tag("zh-CN").unwrap();
/// assert_eq!(locale.language(), "zh");
/// assert_eq!(locale.country(), Some("CN"));
/// assert_eq!(locale.to_bundle_suffix(), "zh_CN");
/// assert_eq!(locale.to_string(), "zh_CN");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Locale {
    language: String,
    country: Option<String>,
    variant: Option<String>,
}

impl Locale {
    /// Returns the root locale, the locale with no language.
    pub fn root() -> Self {
        Self {
            language: String::new(),
            country: None,
            variant: None,
        }
    }

    /// Returns the `en` locale.
    pub fn english() -> Self {
        Self::for_language("en")
    }

    /// Returns the locale of the given language.
    ///
    /// # Arguments
    ///
    /// * `language` - The language, for example `zh`. An empty language returns
    ///   the [root](Self::root) locale.
    pub fn for_language(language: impl Into<String>) -> Self {
        let language = language.into().to_ascii_lowercase();
        if language.is_empty() {
            return Self::root();
        }

        Self {
            language,
            country: None,
            variant: None,
        }
    }

    /// Returns the locale of the given language and country.
    ///
    /// # Arguments
    ///
    /// * `language` - The language, for example `zh`.
    /// * `country` - The country, for example `CN`.
    pub fn new(language: impl Into<String>, country: impl Into<String>) -> Self {
        let mut locale = Self::for_language(language);
        let country = country.into().to_ascii_uppercase();
        if !country.is_empty() {
            locale.country = Some(country);
        }
        locale
    }

    /// Returns this locale with the given variant.
    ///
    /// # Arguments
    ///
    /// * `variant` - The variant, for example `POSIX`.
    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        let variant = variant.into();
        self.variant = (!variant.is_empty()).then_some(variant);
        self
    }

    /// Parses a locale from a language tag such as `zh-CN`, `zh_CN` or `en`.
    ///
    /// Both the BCP 47 form (with `-` separators) and the JDK form (with `_`
    /// separators) are accepted.
    ///
    /// # Errors
    ///
    /// Returns a [`LocaleParseError`] when the tag has more than three parts or
    /// when one of the parts is invalid.
    pub fn for_language_tag(tag: &str) -> Result<Self, LocaleParseError> {
        tag.parse()
    }

    /// Returns the language of this locale, or an empty string for the root
    /// locale.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Returns the country of this locale, when it has one.
    pub fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }

    /// Returns the variant of this locale, when it has one.
    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    /// Returns whether this is the [root](Self::root) locale.
    pub fn is_root(&self) -> bool {
        self.language.is_empty()
    }

    /// Returns the BCP 47 language tag of this locale, for example `zh-CN`.
    ///
    /// The root locale returns an empty string.
    pub fn to_language_tag(&self) -> String {
        let parts = self.parts();
        parts.join("-")
    }

    /// Returns the JDK form of this locale, for example `zh_CN`.
    ///
    /// This is the form [`Display`](fmt::Display) uses, and the form the bundle
    /// file suffixes are built from. The root locale returns an empty string.
    pub fn to_bundle_suffix(&self) -> String {
        let parts = self.parts();
        parts.join("_")
    }

    /// Returns the locale the operating system is configured with.
    ///
    /// The value is read from `LC_ALL`, `LC_MESSAGES` and `LANG`, in that order.
    /// A missing, empty, `C` or `POSIX` value falls back to [`english`](Self::english),
    /// which is also what the JDK does for the `C` locale.
    pub fn system_locale() -> Self {
        for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            let Ok(value) = std::env::var(name) else {
                continue;
            };

            // Strip the modifiers a POSIX locale may carry: `zh_CN.UTF-8@euro`.
            let value = value
                .split('.')
                .next()
                .unwrap_or_default()
                .split('@')
                .next()
                .unwrap_or_default()
                .trim();

            if value.is_empty() || value == "C" || value == "POSIX" {
                continue;
            }

            if let Ok(locale) = Self::for_language_tag(value) {
                return locale;
            }
        }

        Self::english()
    }

    /// Returns the non empty components of this locale, from the most to the
    /// least specific one.
    fn parts(&self) -> Vec<&str> {
        let mut parts = Vec::with_capacity(3);
        if !self.language.is_empty() {
            parts.push(self.language.as_str());
        }
        if let Some(country) = self.country.as_deref() {
            parts.push(country);
        }
        if let Some(variant) = self.variant.as_deref() {
            parts.push(variant);
        }
        parts
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::english()
    }
}

impl fmt::Display for Locale {
    /// Writes the JDK form of the locale, for example `zh_CN`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_bundle_suffix())
    }
}

impl FromStr for Locale {
    type Err = LocaleParseError;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        let tag = tag.trim();
        if tag.is_empty() {
            return Ok(Self::root());
        }

        let parts = tag
            .split(['-', '_'])
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if parts.len() > 3 {
            return Err(LocaleParseError::new(tag, "expected at most 3 parts"));
        }

        let language = parts[0];
        if !language
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        {
            return Err(LocaleParseError::new(
                tag,
                "the language must be alphabetic",
            ));
        }

        let mut locale = Self::for_language(language);

        if let Some(country) = parts.get(1) {
            let valid = (country.len() == 2
                && country
                    .chars()
                    .all(|character| character.is_ascii_alphabetic()))
                || (country.len() == 3
                    && country.chars().all(|character| character.is_ascii_digit()));
            if !valid {
                return Err(LocaleParseError::new(
                    tag,
                    "the country must be two letters or three digits",
                ));
            }
            locale = Self::new(language, *country);
        }

        if let Some(variant) = parts.get(2) {
            if variant.is_empty() {
                return Err(LocaleParseError::new(tag, "the variant is empty"));
            }
            locale = locale.with_variant(*variant);
        }

        Ok(locale)
    }
}

/// Error returned when a locale tag cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleParseError {
    tag: String,
    reason: &'static str,
}

impl LocaleParseError {
    fn new(tag: &str, reason: &'static str) -> Self {
        Self {
            tag: tag.to_owned(),
            reason,
        }
    }

    /// Returns the tag that could not be parsed.
    pub fn tag(&self) -> &str {
        &self.tag
    }
}

impl fmt::Display for LocaleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid locale tag '{}': {}", self.tag, self.reason)
    }
}

impl Error for LocaleParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jdk_and_bcp47_tags() {
        let locale = Locale::for_language_tag("zh_CN").unwrap();
        assert_eq!(locale, Locale::new("zh", "CN"));
        assert_eq!(locale, Locale::for_language_tag("zh-CN").unwrap());
        assert_eq!(locale.to_language_tag(), "zh-CN");
        assert_eq!(locale.to_bundle_suffix(), "zh_CN");
        assert_eq!(locale.country(), Some("CN"));
    }

    #[test]
    fn normalizes_language_and_country() {
        let locale = Locale::new("ZH", "cn");
        assert_eq!(locale.language(), "zh");
        assert_eq!(locale.country(), Some("CN"));
        assert_eq!(locale.variant(), None);
    }

    #[test]
    fn root_locale_has_no_suffix() {
        assert_eq!(Locale::root().to_bundle_suffix(), "");
        assert_eq!(Locale::root().to_language_tag(), "");
        assert!(Locale::root().is_root());
        assert_eq!("".parse::<Locale>().unwrap(), Locale::root());
    }

    #[test]
    fn rejects_invalid_tags() {
        assert!("zh_CN_extra_more".parse::<Locale>().is_err());
        assert!("1zh".parse::<Locale>().is_err());
        assert!("zh_CHN1".parse::<Locale>().is_err());
    }

    #[test]
    fn keeps_variants() {
        let locale = Locale::for_language_tag("en_US_POSIX").unwrap();
        assert_eq!(locale.variant(), Some("POSIX"));
        assert_eq!(locale.to_bundle_suffix(), "en_US_POSIX");
    }
}
