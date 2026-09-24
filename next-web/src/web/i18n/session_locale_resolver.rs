//! Port of `org.springframework.web.servlet.i18n.SessionLocaleResolver`.
//!
//! Resolves the locale of a request from an attribute of its session, falling
//! back to the configured default locale and then to the locale the request asks
//! for.
//!
//! A locale stored in a session belongs to that session, so it is lost when the
//! session ends; this is why the resolver fits an application that keeps
//! sessions for other reasons as well.
//!
//! The session of this framework holds the locale only: the original can carry a
//! time zone next to the locale, but this framework has no time-zone context, so
//! no time-zone attribute is written.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use next_web_context::Locale;
use next_web_core::anys::any_value::AnyValue;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::i18n::{LocaleResolver, LocaleResolverError};
use crate::web::i18n::base_locale_resolver::BaseLocaleResolver;

/// The name of the session attribute holding the locale when none is configured.
///
/// Equivalent to `LOCALE_SESSION_ATTRIBUTE_NAME`.
pub const LOCALE_SESSION_ATTRIBUTE_NAME: &str = "next.web.i18n.SessionLocaleResolver.LOCALE";

/// The function a resolver uses to answer a request it knows no locale for.
///
/// Equivalent to the `defaultLocaleFunction` of the original, which falls back
/// to the configured default locale and then to the locale of the request.
type DefaultLocaleFunction = Arc<dyn Fn(&dyn HttpRequest) -> Locale + Send + Sync>;

/// Resolves the locale of a request from an attribute of its session.
///
/// The type is the counterpart of `SessionLocaleResolver`, and it embeds the
/// [base resolver](BaseLocaleResolver) that holds the default locale.
pub struct SessionLocaleResolver {
    base: BaseLocaleResolver,
    locale_attribute_name: String,
    default_locale_function: Option<DefaultLocaleFunction>,
}

impl SessionLocaleResolver {
    /// Creates a resolver over the [default attribute](LOCALE_SESSION_ATTRIBUTE_NAME).
    pub fn new() -> Self {
        Self {
            base: BaseLocaleResolver::default(),
            locale_attribute_name: LOCALE_SESSION_ATTRIBUTE_NAME.to_owned(),
            default_locale_function: None,
        }
    }

    /// Returns the name of the session attribute holding the locale.
    pub fn locale_attribute_name(&self) -> &str {
        &self.locale_attribute_name
    }

    /// Sets the name of the session attribute holding the locale.
    ///
    /// Equivalent to `setLocaleAttributeName(String)`.
    ///
    /// # Arguments
    ///
    /// * `locale_attribute_name` - The name of the attribute.
    pub fn set_locale_attribute_name(&mut self, locale_attribute_name: impl Into<String>) {
        self.locale_attribute_name = locale_attribute_name.into();
    }

    /// Sets the function answering a request without a usable session attribute.
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
    pub fn resolve_locale_of(&self, request: &dyn HttpRequest) -> Locale {
        self.locale_of_session(request)
            .unwrap_or_else(|| self.default_locale(request))
    }

    /// Reads the locale of the session of the given request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the session is read from.
    fn locale_of_session(&self, request: &dyn HttpRequest) -> Option<Locale> {
        request
            .session()
            .and_then(|session| session.attribute(self.locale_attribute_name.as_str()))
            .and_then(|value| value.as_ref_value::<Locale>().cloned())
    }

    /// Returns the locale a request without a usable session is answered with.
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

impl Deref for SessionLocaleResolver {
    type Target = BaseLocaleResolver;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for SessionLocaleResolver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl LocaleResolver for SessionLocaleResolver {
    fn resolve_locale(&self, request: &dyn HttpRequest) -> Locale {
        self.resolve_locale_of(request)
    }

    /// Stores the given locale of the request in its session.
    ///
    /// Equivalent to `setLocaleContext(HttpServletRequest, HttpServletResponse, LocaleContext)`.
    /// A locale of `None` removes the attribute of the session, which is how the
    /// original clears the stored locale.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the locale belongs to.
    /// * `_response` - The response of the call, which a session does not need.
    /// * `locale` - The locale to store, or `None` to clear the stored locale.
    ///
    /// # Errors
    ///
    /// Returns a [`LocaleResolverError`] when the request has no session the
    /// locale could be stored in.
    fn set_locale(
        &self,
        request: &mut dyn HttpRequest,
        _response: Option<&mut dyn HttpResponse>,
        locale: Option<&Locale>,
    ) -> Result<(), LocaleResolverError> {
        let Some(session) = request.session_mut(true) else {
            return Err(LocaleResolverError::missing_session(
                "The request has no session, the locale cannot be stored",
            ));
        };

        match locale {
            Some(locale) => session.set_attribute(
                self.locale_attribute_name.as_str(),
                AnyValue::BoxedValue(Box::new(locale.clone())),
            ),
            None => session.remove_attribute(self.locale_attribute_name.as_str()),
        }

        Ok(())
    }
}

impl Default for SessionLocaleResolver {
    /// Creates a resolver over the [default attribute](LOCALE_SESSION_ATTRIBUTE_NAME).
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SessionLocaleResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionLocaleResolver")
            .field("base", &self.base)
            .field("locale_attribute_name", &self.locale_attribute_name)
            .field(
                "default_locale_function",
                &self.default_locale_function.is_some(),
            )
            .finish()
    }
}

impl Clone for SessionLocaleResolver {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            locale_attribute_name: self.locale_attribute_name.clone(),
            default_locale_function: self.default_locale_function.clone(),
        }
    }
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
    fn the_locale_attribute_is_configurable() {
        let mut resolver = SessionLocaleResolver::new();
        assert_eq!(
            resolver.locale_attribute_name(),
            LOCALE_SESSION_ATTRIBUTE_NAME
        );

        resolver.set_locale_attribute_name("locale");
        assert_eq!(resolver.locale_attribute_name(), "locale");
    }

    #[test]
    fn the_default_locale_answers_a_request_without_a_session() {
        let mut resolver = SessionLocaleResolver::new();
        resolver.set_default_locale(Some(locale("zh-CN")));

        assert_eq!(
            resolver.resolve_locale_of(&request(Some("de-DE"))),
            locale("zh-CN")
        );

        // Without a default locale the locale of the request is used.
        let resolver = SessionLocaleResolver::new();
        assert_eq!(
            resolver.resolve_locale_of(&request(Some("de-DE"))),
            locale("de-DE")
        );
    }

    #[test]
    fn a_custom_function_answers_a_request_without_a_session() {
        let mut resolver = SessionLocaleResolver::new();
        resolver.set_default_locale(Some(locale("zh-CN")));
        resolver.set_default_locale_function(|_request| locale("ja-JP"));

        assert_eq!(
            resolver.resolve_locale_of(&request(Some("de-DE"))),
            locale("ja-JP")
        );
    }

    #[test]
    fn a_request_without_a_session_cannot_store_a_locale() {
        let resolver = SessionLocaleResolver::new();
        let mut request = request(None);

        let error = resolver
            .set_locale(&mut request, None, Some(&locale("en-US")))
            .unwrap_err();

        assert_eq!(error.kind(), LocaleResolverErrorKind::MissingSession);
        assert!(error.message().contains("no session"));
    }
}
