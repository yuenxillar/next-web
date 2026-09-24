//! Port of `org.springframework.web.servlet.i18n.CookieLocaleResolver`.
//!
//! Resolves the locale of a request from a cookie that is sent back to the
//! client when the locale is changed, falling back to the configured default
//! locale and then to the locale the request asks for.
//!
//! The cookie of this framework holds the locale only: the original can carry a
//! time zone next to the locale, but this framework has no time-zone context, so
//! the time zone part of a cookie is ignored when it is read.
//!
//! The original remembers a locale that was changed while a request is handled
//! in a request attribute; this framework keeps the locale of the current
//! request in [`RequestLocaleHolder`](crate::web::i18n::RequestLocaleHolder)
//! instead, which is what the `#[translation]` macro publishes the resolved
//! locale with.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

use next_web_context::Locale;
use next_web_core::headers::{Cookie as HeaderCookie, HeaderMapExt};
use next_web_core::http::{Cookie, SameSite};
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};
use tracing::{debug, trace};

use crate::i18n::{LocaleResolver, LocaleResolverError};
use crate::web::i18n::base_locale_resolver::BaseLocaleResolver;

/// The name of the cookie holding the locale when none is configured.
///
/// Equivalent to `DEFAULT_COOKIE_NAME`.
pub const DEFAULT_COOKIE_NAME: &str = "next-web-locale";

/// The value a cookie holds when the client asks for no locale at all.
const NO_LOCALE_VALUE: &str = "-";

/// The function a resolver uses to answer a request it knows no locale for.
///
/// Equivalent to the `defaultLocaleFunction` of the original, which falls back
/// to the configured default locale and then to the locale of the request.
type DefaultLocaleFunction = Arc<dyn Fn(&dyn HttpRequest) -> Locale + Send + Sync>;

/// Resolves the locale of a request from a cookie.
///
/// The type is the counterpart of `CookieLocaleResolver`, and it embeds the
/// [base resolver](BaseLocaleResolver) that holds the default locale.
pub struct CookieLocaleResolver {
    base: BaseLocaleResolver,
    cookie_name: String,
    cookie_path: Option<String>,
    cookie_domain: Option<String>,
    cookie_max_age: Option<Duration>,
    cookie_secure: bool,
    cookie_http_only: bool,
    cookie_same_site: Option<SameSite>,
    language_tag_compliant: bool,
    reject_invalid_cookies: bool,
    default_locale_function: Option<DefaultLocaleFunction>,
}

impl CookieLocaleResolver {
    /// Creates a resolver over a cookie with the given name.
    ///
    /// The cookie is sent for the whole application, as a session cookie with
    /// the `SameSite=Lax` attribute, which is the default of the original.
    ///
    /// # Arguments
    ///
    /// * `cookie_name` - The name of the cookie holding the locale.
    pub fn new(cookie_name: impl Into<String>) -> Self {
        let cookie_name = cookie_name.into();

        Self {
            base: BaseLocaleResolver::default(),
            cookie_name,
            cookie_path: Some(String::from("/")),
            cookie_domain: None,
            cookie_max_age: None,
            cookie_secure: false,
            cookie_http_only: false,
            cookie_same_site: Some(SameSite::Lax),
            language_tag_compliant: true,
            reject_invalid_cookies: true,
            default_locale_function: None,
        }
    }

    /// Returns the name of the cookie holding the locale.
    pub fn cookie_name(&self) -> &str {
        &self.cookie_name
    }

    /// Sets the `Max-Age` attribute of the locale cookie.
    ///
    /// Equivalent to `setCookieMaxAge(Duration)`. Without an age the cookie is
    /// a session cookie, which is the default.
    ///
    /// # Arguments
    ///
    /// * `cookie_max_age` - The lifetime of the cookie.
    pub fn set_cookie_max_age(&mut self, cookie_max_age: Duration) {
        self.cookie_max_age = Some(cookie_max_age);
    }

    /// Sets the `Path` attribute of the locale cookie.
    ///
    /// Equivalent to `setCookiePath(String)`. Passing `None` clears the
    /// attribute.
    ///
    /// # Arguments
    ///
    /// * `cookie_path` - The path the cookie is sent for.
    pub fn set_cookie_path(&mut self, cookie_path: Option<impl Into<String>>) {
        self.cookie_path = cookie_path.map(Into::into);
    }

    /// Sets the `Domain` attribute of the locale cookie.
    ///
    /// Equivalent to `setCookieDomain(String)`. Passing `None` clears the
    /// attribute.
    ///
    /// # Arguments
    ///
    /// * `cookie_domain` - The domain the cookie is sent for.
    pub fn set_cookie_domain(&mut self, cookie_domain: Option<impl Into<String>>) {
        self.cookie_domain = cookie_domain.map(Into::into);
    }

    /// Sets the `Secure` attribute of the locale cookie.
    ///
    /// Equivalent to `setCookieSecure(boolean)`.
    ///
    /// # Arguments
    ///
    /// * `cookie_secure` - Whether the cookie is only sent over a secure
    ///   connection.
    pub fn set_cookie_secure(&mut self, cookie_secure: bool) {
        self.cookie_secure = cookie_secure;
    }

    /// Sets the `HttpOnly` attribute of the locale cookie.
    ///
    /// Equivalent to `setCookieHttpOnly(boolean)`.
    ///
    /// # Arguments
    ///
    /// * `cookie_http_only` - Whether the cookie is hidden from scripts.
    pub fn set_cookie_http_only(&mut self, cookie_http_only: bool) {
        self.cookie_http_only = cookie_http_only;
    }

    /// Sets the `SameSite` attribute of the locale cookie.
    ///
    /// Equivalent to `setCookieSameSite(String)`. The default is
    /// [`SameSite::Lax`].
    ///
    /// # Arguments
    ///
    /// * `cookie_same_site` - The cross-site policy of the cookie.
    pub fn set_cookie_same_site(&mut self, cookie_same_site: SameSite) {
        self.cookie_same_site = Some(cookie_same_site);
    }

    /// Returns whether the locale cookie is written as a BCP 47 language tag.
    ///
    /// Equivalent to `isLanguageTagCompliant()`.
    pub fn is_language_tag_compliant(&self) -> bool {
        self.language_tag_compliant
    }

    /// Sets whether the locale cookie is written as a BCP 47 language tag.
    ///
    /// Equivalent to `setLanguageTagCompliant(boolean)`. The default is `true`,
    /// which writes `en-US`; `false` writes the legacy form, `en_US`. A cookie
    /// is read leniently in both forms either way.
    ///
    /// # Arguments
    ///
    /// * `language_tag_compliant` - Whether a language tag is written.
    pub fn set_language_tag_compliant(&mut self, language_tag_compliant: bool) {
        self.language_tag_compliant = language_tag_compliant;
    }

    /// Returns whether a cookie that cannot be read is reported as an error.
    ///
    /// Equivalent to `isRejectInvalidCookies()`.
    pub fn is_reject_invalid_cookies(&self) -> bool {
        self.reject_invalid_cookies
    }

    /// Sets whether a cookie that cannot be read is reported as an error.
    ///
    /// Equivalent to `setRejectInvalidCookies(boolean)`. When this is turned
    /// off, an invalid cookie is logged and the default locale is used instead.
    ///
    /// # Arguments
    ///
    /// * `reject_invalid_cookies` - Whether an invalid cookie is an error.
    pub fn set_reject_invalid_cookies(&mut self, reject_invalid_cookies: bool) {
        self.reject_invalid_cookies = reject_invalid_cookies;
    }

    /// Sets the function answering a request without a usable cookie.
    ///
    /// Equivalent to `setDefaultLocaleFunction(Function)`. Without a function
    /// the resolver uses the configured
    /// [default locale](BaseLocaleResolver::set_default_locale) and then the
    /// locale of the request.
    ///
    /// # Arguments
    ///
    /// * `default_locale_function` - The function returning the fallback locale.
    pub fn set_default_locale_function(
        &mut self,
        default_locale_function: impl Fn(&dyn HttpRequest) -> Locale + Send + Sync + 'static,
    ) {
        self.default_locale_function = Some(Arc::new(default_locale_function));
    }

    /// Resolves the locale of the given request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the locale is resolved for.
    ///
    /// # Errors
    ///
    /// Returns a [`LocaleResolverError`] when the cookie of the request cannot
    /// be read and [invalid cookies are rejected](Self::set_reject_invalid_cookies).
    pub fn try_resolve_locale(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Locale, LocaleResolverError> {
        match self.locale_of_cookie(request)? {
            Some(locale) => {
                trace!("Resolved the locale [{locale}] of the request from its cookie");
                Ok(locale)
            }
            None => Ok(self.default_locale(request)),
        }
    }

    /// Reads the given value of a locale cookie.
    ///
    /// The value may carry a time zone after a `/`, or, for a cookie written by
    /// an older version, after a space; the time zone is not used by this
    /// framework and is therefore ignored. A value of `-` means that the client
    /// asks for no locale at all.
    ///
    /// Equivalent to `parseLocaleValue(String)`.
    ///
    /// # Arguments
    ///
    /// * `locale_value` - The value of the cookie.
    ///
    /// # Errors
    ///
    /// Returns a [`LocaleResolverError`] when the value is not a locale this
    /// framework can read.
    pub fn parse_locale_value(
        &self,
        locale_value: &str,
    ) -> Result<Option<Locale>, LocaleResolverError> {
        let locale_part = locale_value
            .split_once('/')
            .or_else(|| locale_value.split_once(' '))
            .map(|(locale_part, _time_zone)| locale_part)
            .unwrap_or(locale_value)
            .trim();

        if locale_part.is_empty() || locale_part == NO_LOCALE_VALUE {
            return Ok(None);
        }

        // Both the language tag of the original and its legacy form are
        // accepted, whatever `language_tag_compliant` says.
        Locale::for_language_tag(locale_part)
            .map(Some)
            .map_err(|error| {
                LocaleResolverError::invalid_cookie(format!(
                    "Encountered an invalid locale cookie '{locale_value}': {error}"
                ))
            })
    }

    /// Renders the given locale as the value of a locale cookie.
    ///
    /// Equivalent to `toLocaleValue(Locale)`: the language tag `en-US` is
    /// written when [language tags are compliant](Self::set_language_tag_compliant),
    /// the legacy form `en_US` otherwise.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale to render.
    pub fn to_locale_value(&self, locale: &Locale) -> String {
        if self.language_tag_compliant {
            return locale.to_language_tag();
        }

        locale.to_bundle_suffix()
    }

    /// Reads the locale of the cookie of the given request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the cookie is read from.
    ///
    /// # Errors
    ///
    /// Returns a [`LocaleResolverError`] when the cookie cannot be read and
    /// [invalid cookies are rejected](Self::set_reject_invalid_cookies).
    fn locale_of_cookie(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Locale>, LocaleResolverError> {
        let Some(value) = self.cookie_value(request) else {
            return Ok(None);
        };

        match self.parse_locale_value(&value) {
            Ok(locale) => Ok(locale),
            Err(error) if self.reject_invalid_cookies => Err(error),
            Err(error) => {
                debug!(
                    "Ignoring the locale cookie [{}] of the request: {error}",
                    self.cookie_name
                );
                Ok(None)
            }
        }
    }

    /// Returns the value of the locale cookie of the given request, if it has
    /// one.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the cookie is read from.
    fn cookie_value(&self, request: &dyn HttpRequest) -> Option<String> {
        request
            .headers()
            .typed_get::<HeaderCookie>()?
            .iter()
            .find(|(name, _)| *name == self.cookie_name)
            .map(|(_, value)| value.to_owned())
    }

    /// Returns the locale a request without a usable cookie is answered with.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the locale is resolved for.
    fn default_locale(&self, request: &dyn HttpRequest) -> Locale {
        match self.default_locale_function.as_ref() {
            Some(default_locale_function) => default_locale_function(request),
            None => self.base.default_locale_with_request(request),
        }
    }
}

impl Deref for CookieLocaleResolver {
    type Target = BaseLocaleResolver;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CookieLocaleResolver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl LocaleResolver for CookieLocaleResolver {
    /// Resolves the locale of the given request.
    ///
    /// A cookie that cannot be read and that
    /// [is rejected](Self::set_reject_invalid_cookies) is reported as an error
    /// by [`try_resolve_locale`](Self::try_resolve_locale); this method cannot
    /// report it, so it logs the error and answers the request with its default
    /// locale.
    fn resolve_locale(&self, request: &dyn HttpRequest) -> Locale {
        match self.try_resolve_locale(request) {
            Ok(locale) => locale,
            Err(error) => {
                debug!("Falling back to the default locale of the request: {error}");
                self.default_locale(request)
            }
        }
    }

    /// Stores the given locale of the request in the cookie of the response.
    ///
    /// Equivalent to `setLocaleContext(HttpServletRequest, HttpServletResponse, LocaleContext)`.
    /// A locale of `None` is written as a cookie that carries no locale, which
    /// is how the original clears its cookie.
    ///
    /// # Arguments
    ///
    /// * `_request` - The request the locale belongs to, which the original uses
    ///   for its request attribute.
    /// * `response` - The response the cookie is added to.
    /// * `locale` - The locale to store, or `None` to clear the cookie.
    ///
    /// # Errors
    ///
    /// Returns a [`LocaleResolverError`] when the call has no response, or when
    /// the cookie cannot be created, for example because its attributes
    /// contradict each other.
    fn set_locale(
        &self,
        _request: &mut dyn HttpRequest,
        response: Option<&mut dyn HttpResponse>,
        locale: Option<&Locale>,
    ) -> Result<(), LocaleResolverError> {
        let Some(response) = response else {
            return Err(LocaleResolverError::missing_response(format!(
                "A response is required to store the locale in the cookie [{}]",
                self.cookie_name
            )));
        };

        let value = match locale {
            Some(locale) => self.to_locale_value(locale),
            None => NO_LOCALE_VALUE.to_owned(),
        };

        let mut builder = Cookie::builder(self.cookie_name.clone(), Some(value))
            .secure(self.cookie_secure)
            .http_only(self.cookie_http_only);

        if let Some(cookie_path) = self.cookie_path.as_ref() {
            builder = builder.path(cookie_path.clone());
        }
        if let Some(cookie_domain) = self.cookie_domain.as_ref() {
            builder = builder.domain(cookie_domain.clone());
        }
        if let Some(cookie_max_age) = self.cookie_max_age {
            builder = builder.max_age(max_age_of(cookie_max_age));
        }
        if let Some(cookie_same_site) = self.cookie_same_site {
            builder = builder.same_site(cookie_same_site);
        }

        let cookie = builder.build()?;
        response.add_cookie(cookie);

        // A locale that was changed while the request is handled is published
        // with `RequestLocaleHolder`, which is what a later handler reads.
        trace!("Stored the locale of the request in its cookie");

        Ok(())
    }
}

impl Default for CookieLocaleResolver {
    /// Creates a resolver over the [default cookie](DEFAULT_COOKIE_NAME).
    fn default() -> Self {
        Self::new(DEFAULT_COOKIE_NAME)
    }
}

impl fmt::Debug for CookieLocaleResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CookieLocaleResolver")
            .field("base", &self.base)
            .field("cookie_name", &self.cookie_name)
            .field("cookie_path", &self.cookie_path)
            .field("cookie_domain", &self.cookie_domain)
            .field("cookie_max_age", &self.cookie_max_age)
            .field("cookie_secure", &self.cookie_secure)
            .field("cookie_http_only", &self.cookie_http_only)
            .field("cookie_same_site", &self.cookie_same_site)
            .field("language_tag_compliant", &self.language_tag_compliant)
            .field("reject_invalid_cookies", &self.reject_invalid_cookies)
            .field(
                "default_locale_function",
                &self.default_locale_function.is_some(),
            )
            .finish()
    }
}

impl Clone for CookieLocaleResolver {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            cookie_name: self.cookie_name.clone(),
            cookie_path: self.cookie_path.clone(),
            cookie_domain: self.cookie_domain.clone(),
            cookie_max_age: self.cookie_max_age,
            cookie_secure: self.cookie_secure,
            cookie_http_only: self.cookie_http_only,
            cookie_same_site: self.cookie_same_site,
            language_tag_compliant: self.language_tag_compliant,
            reject_invalid_cookies: self.reject_invalid_cookies,
            default_locale_function: self.default_locale_function.clone(),
        }
    }
}

/// Returns the `Max-Age` attribute of a cookie lifetime, saturating at the
/// largest value the attribute can hold.
///
/// # Arguments
///
/// * `duration` - The lifetime of the cookie.
fn max_age_of(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, response::Response};

    use super::*;
    use crate::i18n::LocaleResolverErrorKind;

    /// Returns the locale of the given language tag.
    fn locale(tag: &str) -> Locale {
        Locale::for_language_tag(tag).expect("the test locale is valid")
    }

    /// Returns a request with the given cookie header.
    fn request(cookie: Option<&str>) -> Request {
        let builder = Request::builder();
        let builder = match cookie {
            Some(cookie) => builder.header("Cookie", cookie),
            None => builder,
        };

        builder
            .body(Body::empty())
            .expect("the test request is well formed")
    }

    /// Returns an empty response.
    fn response() -> Response {
        Response::builder()
            .body(Body::empty())
            .expect("the test response is well formed")
    }

    #[test]
    fn renders_and_reads_the_language_tag_of_a_locale() {
        let resolver = CookieLocaleResolver::default();

        assert_eq!(resolver.to_locale_value(&locale("en-US")), "en-US");
        assert_eq!(
            resolver.parse_locale_value("en-US").unwrap(),
            Some(locale("en-US"))
        );
        // The legacy form is accepted as well.
        assert_eq!(
            resolver.parse_locale_value("en_US").unwrap(),
            Some(locale("en-US"))
        );
    }

    #[test]
    fn renders_the_legacy_form_when_language_tags_are_not_compliant() {
        let mut resolver = CookieLocaleResolver::default();
        resolver.set_language_tag_compliant(false);

        assert!(!resolver.is_language_tag_compliant());
        assert_eq!(resolver.to_locale_value(&locale("en-US")), "en_US");
    }

    #[test]
    fn ignores_the_time_zone_of_a_cookie() {
        let resolver = CookieLocaleResolver::default();

        assert_eq!(
            resolver.parse_locale_value("de-DE/Europe/Berlin").unwrap(),
            Some(locale("de-DE"))
        );
        // The legacy form separates the locale from the time zone with a space,
        // which is read before the time zone of the value.
        assert_eq!(
            resolver.parse_locale_value("de-DE UTC").unwrap(),
            Some(locale("de-DE"))
        );
        assert_eq!(resolver.parse_locale_value("-").unwrap(), None);
        assert_eq!(resolver.parse_locale_value("").unwrap(), None);
    }

    #[test]
    fn reports_a_cookie_that_is_not_a_locale() {
        let resolver = CookieLocaleResolver::default();
        let error = resolver.parse_locale_value("%not a locale%").unwrap_err();

        assert_eq!(error.kind(), LocaleResolverErrorKind::InvalidCookie);
        assert!(error.message().contains("%not a locale%"));
    }

    #[test]
    fn resolves_the_locale_of_the_cookie_of_a_request() {
        let resolver = CookieLocaleResolver::default();

        assert_eq!(
            resolver
                .try_resolve_locale(&request(Some("next-web-locale=zh-CN")))
                .unwrap(),
            locale("zh-CN")
        );
        assert_eq!(
            resolver
                .try_resolve_locale(&request(Some("other=1; next-web-locale=ja-JP")))
                .unwrap(),
            locale("ja-JP")
        );
    }

    #[test]
    fn falls_back_to_the_default_locale_without_a_cookie() {
        let mut resolver = CookieLocaleResolver::default();
        resolver.set_default_locale(Some(locale("fr-FR")));

        assert_eq!(
            resolver
                .try_resolve_locale(&request(Some("other=1")))
                .unwrap(),
            locale("fr-FR")
        );

        // Without a default locale the locale of the request is used.
        let resolver = CookieLocaleResolver::default();
        assert_eq!(
            resolver.try_resolve_locale(&request(None)).unwrap(),
            Locale::system_locale()
        );
    }

    #[test]
    fn a_custom_function_answers_a_request_without_a_cookie() {
        let mut resolver = CookieLocaleResolver::default();
        resolver.set_default_locale(Some(locale("fr-FR")));
        resolver.set_default_locale_function(|_request| locale("ja-JP"));

        assert_eq!(
            resolver.try_resolve_locale(&request(None)).unwrap(),
            locale("ja-JP")
        );
    }

    #[test]
    fn rejects_or_ignores_a_cookie_that_cannot_be_read() {
        let resolver = CookieLocaleResolver::default();
        let cookie = request(Some("next-web-locale=not-a-locale"));

        let error = resolver.try_resolve_locale(&cookie).unwrap_err();
        assert_eq!(error.kind(), LocaleResolverErrorKind::InvalidCookie);

        let mut lenient = CookieLocaleResolver::default();
        lenient.set_reject_invalid_cookies(false);
        assert!(!lenient.is_reject_invalid_cookies());
        assert!(lenient.try_resolve_locale(&cookie).is_ok());
    }

    #[test]
    fn stores_the_locale_of_a_request_in_a_cookie() {
        let mut resolver = CookieLocaleResolver::default();
        resolver.set_cookie_max_age(Duration::from_secs(60));
        resolver.set_cookie_path(Some("/app"));
        resolver.set_cookie_secure(true);
        resolver.set_cookie_http_only(true);

        let mut original_request = request(None);
        let mut response = response();
        resolver
            .set_locale(
                &mut original_request,
                Some(&mut response),
                Some(&locale("en-US")),
            )
            .unwrap();

        let set_cookie = response.header("set-cookie").unwrap_or_default();
        assert!(set_cookie.contains("next-web-locale=en-US"));
        assert!(set_cookie.contains("Max-Age=60"));
        assert!(set_cookie.contains("Path=/app"));
        assert!(set_cookie.contains("Secure"));
        assert!(set_cookie.contains("HttpOnly"));

        // The cookie the client sends back is the locale of the next request.
        assert_eq!(
            resolver
                .try_resolve_locale(&request(Some("next-web-locale=en-US")))
                .unwrap(),
            locale("en-US")
        );
    }

    #[test]
    fn clears_the_cookie_of_a_request_without_a_locale() {
        let resolver = CookieLocaleResolver::default();
        let mut original_request = request(None);
        let mut response = response();

        resolver
            .set_locale(&mut original_request, Some(&mut response), None)
            .unwrap();

        let set_cookie = response.header("set-cookie").unwrap_or_default();
        assert!(set_cookie.contains("next-web-locale=-"));
    }

    #[test]
    fn a_call_without_a_response_cannot_store_a_locale() {
        let resolver = CookieLocaleResolver::default();
        let mut original_request = request(None);

        let error = resolver
            .set_locale(&mut original_request, None, Some(&locale("en-US")))
            .unwrap_err();

        assert_eq!(error.kind(), LocaleResolverErrorKind::MissingResponse);
    }

    #[test]
    fn saturates_the_max_age_of_a_cookie() {
        assert_eq!(max_age_of(Duration::from_secs(60)), 60);
        assert_eq!(max_age_of(Duration::from_secs(u64::MAX)), u32::MAX);
    }
}
