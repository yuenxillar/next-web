//! Port of `org.springframework.web.servlet.i18n.AbstractLocaleResolver`.
//!
//! The base of the locale resolvers. It holds the locale that a resolver returns
//! when it finds no other locale, and the concrete resolvers embed it and expose
//! it through [`Deref`], since Rust has no class inheritance.

use std::error::Error;
use std::fmt;

use next_web_context::Locale;
use next_web_core::http::CookieError;
use next_web_core::traits::http::http_request::HttpRequest;

/// The kind of a [`LocaleResolverError`].
///
/// The kind identifies the error without forcing the caller to match on the
/// message, which mirrors the exception types of the original implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LocaleResolverErrorKind {
    /// The resolver cannot change the locale of a request.
    ///
    /// The `Accept-Language` header belongs to the client, so an accept header
    /// resolver rejects every attempt to change it.
    Unsupported,

    /// The value of a locale cookie could not be read as a locale.
    InvalidCookie,

    /// The request has no session the locale could be stored in.
    MissingSession,

    /// The locale cannot be stored because the call has no response.
    MissingResponse,
}

impl fmt::Display for LocaleResolverErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocaleResolverErrorKind::Unsupported => f.write_str("unsupported operation"),
            LocaleResolverErrorKind::InvalidCookie => f.write_str("invalid locale cookie"),
            LocaleResolverErrorKind::MissingSession => f.write_str("missing session"),
            LocaleResolverErrorKind::MissingResponse => f.write_str("missing response"),
        }
    }
}

/// Error returned by a locale resolver.
///
/// The error carries the [`kind`](Self::kind) of the failure next to its
/// message, so that a caller can react to a specific failure the same way the
/// original implementation reacts to a specific exception.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleResolverError {
    kind: LocaleResolverErrorKind,
    message: String,
}

impl LocaleResolverError {
    /// Creates an error of the given kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of the failure.
    /// * `message` - A message describing the failure.
    pub fn new(kind: LocaleResolverErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates the error of an operation a resolver cannot perform.
    ///
    /// # Arguments
    ///
    /// * `message` - A message describing why the operation is not supported.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(LocaleResolverErrorKind::Unsupported, message)
    }

    /// Creates the error of a locale cookie that could not be read.
    ///
    /// # Arguments
    ///
    /// * `message` - A message describing the value that could not be read.
    pub fn invalid_cookie(message: impl Into<String>) -> Self {
        Self::new(LocaleResolverErrorKind::InvalidCookie, message)
    }

    /// Creates the error of a request without the session a locale needs.
    ///
    /// # Arguments
    ///
    /// * `message` - A message describing the missing session.
    pub fn missing_session(message: impl Into<String>) -> Self {
        Self::new(LocaleResolverErrorKind::MissingSession, message)
    }

    /// Creates the error of a call that has no response to store a locale in.
    ///
    /// # Arguments
    ///
    /// * `message` - A message describing the response that is required.
    pub fn missing_response(message: impl Into<String>) -> Self {
        Self::new(LocaleResolverErrorKind::MissingResponse, message)
    }

    /// Returns the kind of this error.
    pub fn kind(&self) -> LocaleResolverErrorKind {
        self.kind
    }

    /// Returns the message of this error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LocaleResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl Error for LocaleResolverError {}

impl From<CookieError> for LocaleResolverError {
    /// Converts the error of a cookie that could not be built into an error of
    /// the locale the resolver tried to store.
    fn from(error: CookieError) -> Self {
        Self::invalid_cookie(format!("Locale cookie could not be created: {error}"))
    }
}

/// The base of the locale resolvers, holding the default locale.
///
/// The type is the counterpart of `AbstractLocaleResolver`: a resolver embeds it
/// and dereferences to it, so that `set_default_locale` and `default_locale` are
/// available on every resolver.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseLocaleResolver {
    default_locale: Option<Locale>,
}

impl BaseLocaleResolver {
    /// Creates a base resolver without a default locale.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the locale used when the resolver finds no other locale.
    ///
    /// Equivalent to `getDefaultLocale()`.
    pub fn default_locale(&self) -> Option<Locale> {
        self.default_locale.clone()
    }

    /// Sets the locale used when the resolver finds no other locale.
    ///
    /// Equivalent to `setDefaultLocale(Locale)`. Passing `None` restores the
    /// behavior of the original, which is to fall back to the locale of the
    /// request.
    ///
    /// # Arguments
    ///
    /// * `default_locale` - The locale to fall back to.
    pub fn set_default_locale(&mut self, default_locale: Option<Locale>) {
        self.default_locale = default_locale;
    }

    /// Returns the locale a request falls back to.
    ///
    /// This is the function the original installs as its default
    /// `defaultLocaleFunction`: the configured default locale when there is one,
    /// and the locale the request asks for otherwise.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the fallback locale is resolved for.
    pub fn default_locale_with_request(&self, request: &dyn HttpRequest) -> Locale {
        self.default_locale
            .clone()
            .or_else(|| request.locale())
            .unwrap_or_else(Locale::system_locale)
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::*;

    /// Returns the locale of the given language tag.
    fn locale(tag: &str) -> Locale {
        Locale::for_language_tag(tag).expect("the test locale is valid")
    }

    /// Returns a request with the given `Accept-Language` header.
    fn request(accept_language: Option<&str>) -> Request {
        let builder = Request::builder();
        let builder = match accept_language {
            Some(accept_language) => builder.header("Accept-Language", accept_language),
            None => builder,
        };

        builder
            .body(Body::empty())
            .expect("the test request is well formed")
    }

    #[test]
    fn the_default_locale_wins_over_the_request_locale() {
        let mut resolver = BaseLocaleResolver::new();
        assert_eq!(resolver.default_locale(), None);

        resolver.set_default_locale(Some(locale("ja-JP")));
        assert_eq!(resolver.default_locale(), Some(locale("ja-JP")));
        assert_eq!(
            resolver.default_locale_with_request(&request(Some("de-DE"))),
            locale("ja-JP")
        );
    }

    #[test]
    fn the_request_locale_is_used_without_a_default_locale() {
        let resolver = BaseLocaleResolver::new();

        assert_eq!(
            resolver.default_locale_with_request(&request(Some("zh-CN"))),
            locale("zh-CN")
        );
        // Without a header the locale of the system is used.
        assert_eq!(
            resolver.default_locale_with_request(&request(None)),
            Locale::system_locale()
        );
    }

    #[test]
    fn the_error_keeps_its_kind() {
        let error = LocaleResolverError::invalid_cookie("Unsupported locale cookie [xx]");

        assert_eq!(error.kind(), LocaleResolverErrorKind::InvalidCookie);
        assert!(error.message().contains("[xx]"));
        assert!(error.to_string().starts_with("invalid locale cookie"));

        let error = LocaleResolverError::from(CookieError::SameSiteNoneRequiresSecure);
        assert_eq!(error.kind(), LocaleResolverErrorKind::InvalidCookie);
        assert!(
            error
                .message()
                .contains("Locale cookie could not be created")
        );

        let error = LocaleResolverError::missing_response("A response is required");
        assert_eq!(error.kind(), LocaleResolverErrorKind::MissingResponse);
    }
}
