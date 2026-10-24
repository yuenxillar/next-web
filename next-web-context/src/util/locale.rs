//! Port of `java.util.Locale`, limited to what the message source needs.
//!
//! A locale is the combination of a language and, optionally, a country and a
//! variant, exactly like the JDK models it. The values are used to derive the
//! resource bundle suffixes of a basename, so `zh_CN` resolves to
//! `messages_zh_CN.properties` and falls back to `messages_zh.properties`.
//!
//! The type is shared by every module that resolves the locale of a request,
//! and it is also where an `Accept-Language` header is read:
//! [`Locale::parse_accept_language`] orders the languages of a header by the
//! quality their client declares, and [`Locale::from_accept_language`] returns
//! the most preferred language of a header.
//!
//! The parts of a locale are [`Cow`]s, so the locales that are declared with
//! literals, such as the [known locales](Locale::all_locales) of the framework,
//! do not allocate. A locale that is parsed from a header or read from the
//! system configuration owns its parts.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

/// Declares the locales the framework knows by name, together with the list
/// [`Locale::all_locales`] hands out.
macro_rules! known_locales {
    ( $( $tag:literal => $name:ident ( $language:literal, $country:expr ) ),* $(,)? ) => {
        impl Locale {
            $(
                #[doc = concat!("The `", $tag, "` locale.")]
                pub const $name: Locale = Locale::known($language, $country, None);
            )*
        }

        /// The locales the framework knows by name.
        const KNOWN_LOCALES: &[Locale] = &[ $( Locale::$name ),* ];
    };
}

known_locales! {
    "zh-CN" => ZH_CN ("zh", Some("CN")),
    "zh-TW" => ZH_TW ("zh", Some("TW")),
    "zh-HK" => ZH_HK ("zh", Some("HK")),
    "en-HK" => EN_HK ("en", Some("HK")),
    "en-US" => EN_US ("en", Some("US")),
    "en-GB" => EN_GB ("en", Some("GB")),
    "en-WW" => EN_WW ("en", Some("WW")),
    "en-CA" => EN_CA ("en", Some("CA")),
    "en-AU" => EN_AU ("en", Some("AU")),
    "en-IE" => EN_IE ("en", Some("IE")),
    "en-FI" => EN_FI ("en", Some("FI")),
    "en-DK" => EN_DK ("en", Some("DK")),
    "en-IL" => EN_IL ("en", Some("IL")),
    "en-ZA" => EN_ZA ("en", Some("ZA")),
    "en-IN" => EN_IN ("en", Some("IN")),
    "en-NO" => EN_NO ("en", Some("NO")),
    "en-SG" => EN_SG ("en", Some("SG")),
    "en-NZ" => EN_NZ ("en", Some("NZ")),
    "en-ID" => EN_ID ("en", Some("ID")),
    "en-PH" => EN_PH ("en", Some("PH")),
    "en-TH" => EN_TH ("en", Some("TH")),
    "en-MY" => EN_MY ("en", Some("MY")),
    "en-XA" => EN_XA ("en", Some("XA")),
    "ko-KR" => KO_KR ("ko", Some("KR")),
    "ja-JP" => JA_JP ("ja", Some("JP")),
    "nl-NL" => NL_NL ("nl", Some("NL")),
    "nl-BE" => NL_BE ("nl", Some("BE")),
    "pt-PT" => PT_PT ("pt", Some("PT")),
    "pt-BR" => PT_BR ("pt", Some("BR")),
    "fr-FR" => FR_FR ("fr", Some("FR")),
    "fr-LU" => FR_LU ("fr", Some("LU")),
    "fr-CH" => FR_CH ("fr", Some("CH")),
    "fr-BE" => FR_BE ("fr", Some("BE")),
    "fr-CA" => FR_CA ("fr", Some("CA")),
    "es-LA" => ES_LA ("es", Some("LA")),
    "es-ES" => ES_ES ("es", Some("ES")),
    "es-AR" => ES_AR ("es", Some("AR")),
    "es-US" => ES_US ("es", Some("US")),
    "es-MX" => ES_MX ("es", Some("MX")),
    "es-CO" => ES_CO ("es", Some("CO")),
    "es-PR" => ES_PR ("es", Some("PR")),
    "de-DE" => DE_DE ("de", Some("DE")),
    "de-AT" => DE_AT ("de", Some("AT")),
    "de-CH" => DE_CH ("de", Some("CH")),
    "ru-RU" => RU_RU ("ru", Some("RU")),
    "it-IT" => IT_IT ("it", Some("IT")),
    "el-GR" => EL_GR ("el", Some("GR")),
    "no-NO" => NO_NO ("no", Some("NO")),
    "hu-HU" => HU_HU ("hu", Some("HU")),
    "tr-TR" => TR_TR ("tr", Some("TR")),
    "cs-CZ" => CS_CZ ("cs", Some("CZ")),
    "sl-SI" => SL_SI ("sl", Some("SI")),
    "pl-PL" => PL_PL ("pl", Some("PL")),
    "sv-SE" => SV_SE ("sv", Some("SE")),
    "fi-FI" => FI_FI ("fi", Some("FI")),
    "da-DK" => DA_DK ("da", Some("DK")),
    "he-IL" => HE_IL ("he", Some("IL")),
}

/// A language, optionally combined with a country and a variant.
///
/// The language is stored in lower case and the country in upper case, so the
/// value is normalized no matter how it was constructed. The root locale has no
/// language, no country and no variant; it maps to the locale independent
/// bundle, `messages.properties` for the basename `messages`.
///
/// The parts are [`Cow`]s, so a locale that is built from literals borrows them
/// and does not allocate. [`Locale::known`] declares such a locale, which is how
/// the [known locales](Locale::all_locales) of the framework are built, and how
/// an application can declare its own constants.
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
///
/// assert_eq!(locale, Locale::ZH_CN);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Locale {
    language: Cow<'static, str>,
    country: Option<Cow<'static, str>>,
    variant: Option<Cow<'static, str>>,
}

impl Locale {
    /// Returns a locale that borrows the given parts.
    ///
    /// The parts are used as they are, so the caller is responsible for the case
    /// of the language and the country. This is the constructor the constants of
    /// the framework are declared with, and it is public so that an application
    /// can declare its own:
    ///
    /// ```rust
    /// use next_web_context::Locale;
    ///
    /// const PT_PT: Locale = Locale::known("pt", Some("PT"), None);
    ///
    /// assert_eq!(PT_PT.language(), "pt");
    /// assert_eq!(PT_PT.country(), Some("PT"));
    /// assert_eq!(PT_PT.to_bundle_suffix(), "pt_PT");
    /// ```
    pub const fn known(
        language: &'static str,
        country: Option<&'static str>,
        variant: Option<&'static str>,
    ) -> Self {
        Self {
            language: Cow::Borrowed(language),
            country: match country {
                Some(country) => Some(Cow::Borrowed(country)),
                None => None,
            },
            variant: match variant {
                Some(variant) => Some(Cow::Borrowed(variant)),
                None => None,
            },
        }
    }

    /// Returns the locales the framework knows by name.
    ///
    /// The locales are the constants of [`Locale`], from the most to the least
    /// commonly used one. They are the locales the framework offers to an
    /// application that lets a user pick one, for example.
    pub fn all_locales() -> &'static [Locale] {
        KNOWN_LOCALES
    }

    /// Returns the root locale, the locale with no language.
    pub fn root() -> Self {
        Self {
            language: Cow::Borrowed(""),
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
    pub fn for_language(language: impl Into<Cow<'static, str>>) -> Self {
        let language = lowercased(language.into());
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
    pub fn new(
        language: impl Into<Cow<'static, str>>,
        country: impl Into<Cow<'static, str>>,
    ) -> Self {
        let mut locale = Self::for_language(language);
        let country = uppercased(country.into());
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
    pub fn with_variant(mut self, variant: impl Into<Cow<'static, str>>) -> Self {
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
}

impl Locale {
    /// Returns the locales an `Accept-Language` header asks for, from the most
    /// to the least preferred one.
    ///
    /// The quality a client declares for a language is honored, and the
    /// languages without a quality are ranked by the order they appear in. A
    /// language a client rejects with a quality of zero, and a language tag the
    /// framework cannot read, such as `*`, are skipped.
    ///
    /// # Arguments
    ///
    /// * `header` - The value of an `Accept-Language` header.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use next_web_context::Locale;
    ///
    /// let locales = Locale::parse_accept_language("en-US;q=0.5, zh-CN;q=0.9, *");
    ///
    /// assert_eq!(locales, [Locale::ZH_CN, Locale::EN_US]);
    /// ```
    pub fn parse_accept_language(header: &str) -> Vec<Locale> {
        let mut preferences: Vec<(usize, f32, Locale)> = Vec::new();

        for (index, part) in header.split(',').enumerate() {
            let mut quality = 1.0_f32;
            let mut tag = part.trim();

            if let Some((value, options)) = tag.split_once(';') {
                tag = value.trim();

                for option in options.split(';') {
                    let Some((name, value)) = option.split_once('=') else {
                        continue;
                    };

                    if name.trim().eq_ignore_ascii_case("q")
                        && let Ok(parsed) = value.trim().parse::<f32>()
                        && (0.0..=1.0).contains(&parsed)
                    {
                        quality = parsed;
                    }
                }
            }

            if tag.is_empty() || quality <= 0.0 {
                continue;
            }

            if let Ok(locale) = Self::for_language_tag(tag) {
                preferences.push((index, quality, locale));
            }
        }

        preferences.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then(left.0.cmp(&right.0))
        });

        preferences
            .into_iter()
            .map(|(_, _, locale)| locale)
            .collect()
    }

    /// Returns the most preferred locale of an `Accept-Language` header.
    ///
    /// This is the first locale [`Locale::parse_accept_language`] returns, or
    /// `None` when the header asks for no locale the framework can read.
    ///
    /// # Arguments
    ///
    /// * `header` - The value of an `Accept-Language` header.
    pub fn from_accept_language(header: impl AsRef<str>) -> Option<Locale> {
        Self::parse_accept_language(header.as_ref())
            .into_iter()
            .next()
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
    /// A missing, empty, `C` or `POSIX` value falls back to
    /// [`english`](Self::english), which is also what the JDK does for the `C`
    /// locale.
    ///
    /// The locale of the system does not change while the process runs, so the
    /// value is read once and handed out afterwards.
    pub fn system_locale() -> Locale {
        static SYSTEM_LOCALE: OnceLock<Locale> = OnceLock::new();

        SYSTEM_LOCALE.get_or_init(Self::read_system_locale).clone()
    }

    /// Reads the locale the operating system is configured with.
    fn read_system_locale() -> Locale {
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
            parts.push(self.language.as_ref());
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

        let mut locale = Self::for_language(String::from(language));

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
            locale = Self::new(String::from(language), String::from(*country));
        }

        if let Some(variant) = parts.get(2) {
            if variant.is_empty() {
                return Err(LocaleParseError::new(tag, "the variant is empty"));
            }
            locale = locale.with_variant(String::from(*variant));
        }

        Ok(locale)
    }
}

/// Returns the value with its ASCII letters lowercased.
///
/// The value is returned as it is when it is lowercased already, so a borrowed
/// value stays borrowed.
fn lowercased(value: Cow<'static, str>) -> Cow<'static, str> {
    if value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Cow::Owned(value.to_ascii_lowercase())
    } else {
        value
    }
}

/// Returns the value with its ASCII letters uppercased.
///
/// The value is returned as it is when it is uppercased already, so a borrowed
/// value stays borrowed.
fn uppercased(value: Cow<'static, str>) -> Cow<'static, str> {
    if value.bytes().any(|byte| byte.is_ascii_lowercase()) {
        Cow::Owned(value.to_ascii_uppercase())
    } else {
        value
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
    fn the_locales_of_literals_are_equal_to_the_parsed_ones() {
        assert_eq!(Locale::ZH_CN, Locale::for_language_tag("zh-CN").unwrap());
        assert_eq!(Locale::EN_US, Locale::for_language_tag("en-US").unwrap());
        assert_eq!(Locale::english(), Locale::default());
        assert_eq!(Locale::SL_SI.to_language_tag(), "sl-SI");
    }

    #[test]
    fn all_locales_lists_the_known_locales() {
        let locales = Locale::all_locales();

        assert!(locales.contains(&Locale::EN_US));
        assert!(locales.contains(&Locale::ZH_CN));
        assert_eq!(locales.len(), 57);
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

    #[test]
    fn orders_the_requested_locales_by_quality() {
        assert_eq!(
            Locale::parse_accept_language("en-US;q=0.5, zh-CN;q=0.9, de-DE"),
            [Locale::DE_DE, Locale::ZH_CN, Locale::EN_US]
        );
        assert_eq!(
            Locale::parse_accept_language("zh-CN;q=0, en-US"),
            [Locale::EN_US],
            "a language the client rejects is not requested"
        );
        assert!(Locale::parse_accept_language("  ").is_empty());
        assert!(
            Locale::parse_accept_language("*").is_empty(),
            "a wildcard names no locale"
        );
    }

    #[test]
    fn from_accept_language_returns_the_most_preferred_locale() {
        assert_eq!(
            Locale::from_accept_language("en-US;q=0.5, zh-CN;q=0.9"),
            Some(Locale::ZH_CN)
        );
        assert_eq!(Locale::from_accept_language("*"), None);
        assert_eq!(Locale::from_accept_language(""), None);
    }
}
