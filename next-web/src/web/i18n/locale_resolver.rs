use next_web_context::Locale;
use next_web_core::http::{HttpRequest, HttpResponse};

use crate::i18n::LocaleResolverError;

/// Strategy interface for resolving the current locale from a request, and for
/// modifying the locale through a request and response pair.
///
/// Implementations may base their decisions on the request, session, cookies,
/// and other request-scoped state. A typical default implementation simply
/// uses the locale provided by the request's `Accept-Language` header.
///
/// To retrieve the current locale in controllers or views independent of the
/// concrete resolution strategy, use the locale held in the request context.
///
/// An extended strategy interface may additionally resolve a locale context
/// object that carries associated time zone information. Implementations
/// should provide that extended interface wherever appropriate.
/// A resolver is registered as a singleton of the application context and is
/// used by the layer that prepares a request, which runs on the worker threads
/// of the runtime, so it has to be shared between threads and moved between
/// them.
pub trait LocaleResolver
where
    Self: Send + Sync,
{
    /// Resolves the current locale from the given request.
    ///
    /// May return a default locale as a fallback in any case.
    ///
    /// # Arguments
    /// * `request` - the request to resolve the locale for.
    ///
    /// # Returns
    /// The current locale. Implementations must never return `None`; when no
    /// locale can be determined from the request, a default locale should be
    /// returned instead.
    fn resolve_locale(&self, request: &dyn HttpRequest) -> Locale;

    /// Sets the current locale to the given one.
    ///
    /// # Arguments
    /// * `request` - the request used for locale modification.
    /// * `response` - the response used for locale modification, if any.
    /// * `locale` - the new locale, or `None` to clear the locale.
    ///
    /// # Errors
    /// Returns [`LocaleResolverError::Unsupported`] if the implementation does
    /// not support dynamically changing the locale.
    fn set_locale(
        &self,
        request: &mut dyn HttpRequest,
        response: Option<&mut dyn HttpResponse>,
        locale: Option<&Locale>,
    ) -> Result<(), LocaleResolverError>;
}
