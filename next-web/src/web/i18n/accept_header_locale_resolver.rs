//! Port of `org.springframework.web.servlet.i18n.AcceptHeaderLocaleResolver`.
//!
//! Resolves the locale of a request by matching the locales of its
//! `Accept-Language` header against the
//! [supported locales](AcceptHeaderLocaleResolver::set_supported_locales) of the
//! resolver.
//!
//! The locale cannot be changed: the header belongs to the client, so
//! [`set_locale`](LocaleResolver::set_locale) always reports an
//! [unsupported operation](crate::i18n::LocaleResolverErrorKind::Unsupported).

use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};

use next_web_context::Locale;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::i18n::{LocaleResolver, LocaleResolverError};
use crate::web::i18n::base_locale_resolver::BaseLocaleResolver;

/// The header the requested locales are read from.
const ACCEPT_LANGUAGE: &str = "Accept-Language";

/// Resolves the locale of a request against a list of supported locales.
///
/// The type is the counterpart of `AcceptHeaderLocaleResolver`, and it embeds
/// the [base resolver](BaseLocaleResolver) that holds the default locale.
#[derive(Debug, Clone, Default)]
pub struct AcceptHeaderLocaleResolver {
    base: BaseLocaleResolver,
    supported_locales: Vec<Locale>,
}

impl Deref for AcceptHeaderLocaleResolver {
    type Target = BaseLocaleResolver;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for AcceptHeaderLocaleResolver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl AcceptHeaderLocaleResolver {
    /// Creates a resolver without supported locales.
    ///
    /// A resolver without supported locales accepts the locale a request asks
    /// for, which is the default of the original as well.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the locales a request is matched against.
    ///
    /// A supported locale is matched on its language and its country. A
    /// supported locale without a country matches every locale of its language,
    /// which is the language-only fallback of the original: adding
    /// `Locale::for_language("en")` accepts a request for any English locale.
    ///
    /// # Arguments
    ///
    /// * `supported_locales` - The locales the resolver accepts.
    pub fn set_supported_locales(&mut self, supported_locales: impl IntoIterator<Item = Locale>) {
        self.supported_locales = supported_locales.into_iter().collect();
    }

    /// Returns the locales a request is matched against.
    ///
    /// Equivalent to `getSupportedLocales()`.
    pub fn supported_locales(&self) -> &[Locale] {
        &self.supported_locales
    }

    /// Resolves the locale of the given request.
    ///
    /// The locale of the request is returned when no locale is supported, when
    /// the request asks for a supported locale, or when no supported locale
    /// matches. Otherwise the [default locale](BaseLocaleResolver::set_default_locale)
    /// is returned when there is one.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the locale is resolved for.
    pub fn resolve_locale_of(&self, request: &dyn HttpRequest) -> Locale {
        let accept_language = request
            .header(ACCEPT_LANGUAGE)
            .map(str::trim)
            .filter(|header| !header.is_empty());
        let default_locale = self.default_locale();

        // A request without a header is answered with the default locale, like
        // the original does.
        if let (Some(default_locale), None) = (default_locale.as_ref(), accept_language) {
            return default_locale.clone();
        }

        let request_locales = request_locales(accept_language.unwrap_or_default());
        let request_locale = request_locales
            .first()
            .cloned()
            .unwrap_or_else(Locale::system_locale);

        if self.supported_locales.is_empty() || self.supported_locales.contains(&request_locale) {
            return request_locale;
        }

        if let Some(supported_locale) = self.find_supported_locale(&request_locales) {
            return supported_locale;
        }

        default_locale.unwrap_or(request_locale)
    }

    /// Returns the locale a request is answered with when its preferred locales
    /// are not supported.
    ///
    /// Equivalent to `findSupportedLocale(HttpServletRequest, List)`: the first
    /// requested locale that is supported wins, and a supported locale without a
    /// country counts as a match for its language until a supported locale of the
    /// same language is asked for.
    ///
    /// # Arguments
    ///
    /// * `request_locales` - The locales the request asks for, from the most to
    ///   the least preferred one.
    fn find_supported_locale(&self, request_locales: &[Locale]) -> Option<Locale> {
        let mut language_match: Option<Locale> = None;

        for locale in request_locales {
            if self
                .supported_locales
                .iter()
                .any(|supported| supported == locale)
            {
                match language_match.as_ref() {
                    // A language-only match of another language was found first,
                    // so the locales of this language are not preferred.
                    Some(matched) if matched.language() != locale.language() => continue,
                    _ => return Some(locale.clone()),
                }
            } else if language_match.is_none() {
                // Let's try to find a language-only match as a fallback.
                language_match = self
                    .supported_locales
                    .iter()
                    .find(|supported| {
                        supported.country().is_none() && supported.language() == locale.language()
                    })
                    .cloned();
            }
        }

        language_match
    }
}

impl LocaleResolver for AcceptHeaderLocaleResolver {
    fn resolve_locale(&self, request: &dyn HttpRequest) -> Locale {
        self.resolve_locale_of(request)
    }

    /// Reports that the locale of a request cannot be changed.
    ///
    /// Equivalent to `setLocale(HttpServletRequest, HttpServletResponse, Locale)`,
    /// which always throws because the `Accept-Language` header can only be
    /// changed by the client.
    fn set_locale(
        &self,
        _request: &mut dyn HttpRequest,
        _response: Option<&mut dyn HttpResponse>,
        _locale: Option<&Locale>,
    ) -> Result<(), LocaleResolverError> {
        Err(LocaleResolverError::unsupported(
            "Cannot change the Accept-Language header of a request, use another locale resolution strategy",
        ))
    }
}

/// Returns the locales the given `Accept-Language` header asks for, from the
/// most to the least preferred one.
///
/// The quality a client declares for a language is honored, and the languages
/// without a quality are ranked by the order they appear in. A language tag this
/// framework cannot read, such as `*`, is skipped.
///
/// # Arguments
///
/// * `accept_language` - The value of the `Accept-Language` header.
fn request_locales(accept_language: &str) -> Vec<Locale> {
    let mut preferences: Vec<(usize, f32, Locale)> = Vec::new();

    for (index, part) in accept_language.split(',').enumerate() {
        let mut quality = 1.0_f32;
        let mut tag = part.trim();

        if let Some((value, options)) = tag.split_once(';') {
            tag = value.trim();

            for option in options.split(';') {
                let Some((name, value)) = option.split_once('=') else {
                    continue;
                };

                if name.trim().eq_ignore_ascii_case("q") {
                    if let Ok(parsed) = value.trim().parse::<f32>() {
                        if (0.0..=1.0).contains(&parsed) {
                            quality = parsed;
                        }
                    }
                }
            }
        }

        if tag.is_empty() || quality <= 0.0 {
            continue;
        }

        if let Ok(locale) = Locale::for_language_tag(tag) {
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

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::*;
    use crate::i18n::LocaleResolverErrorKind;

    /// Returns the locale of the given language tag.
    fn locale(tag: &str) -> Locale {
        Locale::for_language_tag(tag).expect("the test locale is valid")
    }

    /// Returns a request with the given `Accept-Language` header.
    fn request(accept_language: &str) -> Request {
        Request::builder()
            .header(ACCEPT_LANGUAGE, accept_language)
            .body(Body::empty())
            .expect("the test request is well formed")
    }

    /// Returns a request without an `Accept-Language` header.
    fn request_without_header() -> Request {
        Request::builder()
            .body(Body::empty())
            .expect("the test request is well formed")
    }

    #[test]
    fn orders_the_requested_locales_by_quality() {
        assert_eq!(
            request_locales("en-US;q=0.5, zh-CN;q=0.9, de-DE"),
            [locale("de-DE"), locale("zh-CN"), locale("en-US")]
        );
        assert_eq!(
            request_locales("zh-CN;q=0, en-US"),
            [locale("en-US")],
            "a language the client rejects is not requested"
        );
        assert_eq!(request_locales("  "), []);
        assert_eq!(request_locales("*"), [], "a wildcard names no locale");
    }

    #[test]
    fn the_locale_of_the_request_is_used_without_supported_locales() {
        let resolver = AcceptHeaderLocaleResolver::new();

        assert_eq!(
            resolver.resolve_locale_of(&request("de-DE")),
            locale("de-DE")
        );
        assert_eq!(
            resolver.resolve_locale_of(&request_without_header()),
            Locale::system_locale()
        );
    }

    #[test]
    fn a_supported_locale_is_matched_on_language_and_country() {
        let mut resolver = AcceptHeaderLocaleResolver::new();
        resolver.set_supported_locales([locale("de-DE"), locale("en-US")]);

        assert_eq!(
            resolver.supported_locales(),
            [locale("de-DE"), locale("en-US")]
        );
        assert_eq!(
            resolver.resolve_locale_of(&request("de-DE")),
            locale("de-DE")
        );
        // The client prefers the German locale of Austria, which is not
        // supported, so the supported locale of the next preference wins.
        assert_eq!(
            resolver.resolve_locale_of(&request("de-AT, en-US;q=0.9")),
            locale("en-US")
        );
    }

    #[test]
    fn a_supported_locale_without_a_country_matches_its_language() {
        let mut resolver = AcceptHeaderLocaleResolver::new();
        resolver.set_supported_locales([locale("en"), locale("de-DE")]);

        // `en-GB` is not supported, but the support of `en` accepts it.
        assert_eq!(resolver.resolve_locale_of(&request("en-GB")), locale("en"));
    }

    #[test]
    fn the_default_locale_is_used_without_a_match() {
        let mut resolver = AcceptHeaderLocaleResolver::new();
        resolver.set_supported_locales([locale("de-DE")]);

        assert_eq!(
            resolver.resolve_locale_of(&request("en-GB")),
            locale("en-GB"),
            "without a default locale the request locale is returned"
        );

        resolver.set_default_locale(Some(locale("fr-FR")));
        assert_eq!(
            resolver.resolve_locale_of(&request("en-GB")),
            locale("fr-FR")
        );
    }

    #[test]
    fn the_default_locale_answers_a_request_without_a_header() {
        let mut resolver = AcceptHeaderLocaleResolver::new();
        resolver.set_default_locale(Some(locale("fr-FR")));

        assert_eq!(
            resolver.resolve_locale_of(&request_without_header()),
            locale("fr-FR")
        );
        // A header that asks for a supported locale still wins.
        assert_eq!(
            resolver.resolve_locale_of(&request("de-DE")),
            locale("de-DE")
        );
    }

    #[test]
    fn the_locale_of_a_request_cannot_be_changed() {
        let resolver = AcceptHeaderLocaleResolver::new();
        let mut request = request_without_header();
        let error = resolver
            .set_locale(&mut request, None, Some(&locale("en-US")))
            .unwrap_err();

        assert_eq!(error.kind(), LocaleResolverErrorKind::Unsupported);
    }
}
