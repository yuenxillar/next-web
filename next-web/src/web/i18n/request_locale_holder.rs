//! The locale of the request that is being handled.
//!
//! The locale of a request is kept by the
//! [`LocaleContextHolder`] of the context layer, which is the single place the
//! framework reads it from: a message source that is given no locale looks there,
//! and so does a
//! [`MessageSourceAccessor`](next_web_context::support::MessageSourceAccessor).
//! This type is the view of a handler on the same locale: it answers the locale
//! of the request and nothing else, so a caller can tell a request that carries
//! one from a task that runs outside a request.

use next_web_context::Locale;
use next_web_context::i18n::LocaleContextHolder;

/// Provides access to the locale of the request that is being handled.
///
/// The type is a namespace, not a value: all its methods are associated
/// functions, and they read the locale the layer that prepared the request
/// associated with it, see
/// [`LocaleContextHolder::scope_locale`].
///
/// # Lifecycle
///
/// The locale is associated with the request while it is handled and is dropped
/// when the request is answered. Code that runs outside such a scope — a
/// background task, a test, or a handler of an application that does not build
/// the i18n support — sees no locale and uses one of the defaulting accessors
/// below.
pub struct RequestLocaleHolder;

impl RequestLocaleHolder {
    /// Returns the locale of the request, if the code that runs has one.
    ///
    /// Returns `None` outside a request, so the absence of a locale can be
    /// handled explicitly. Unlike [`Locale::system_locale`], this answers no
    /// locale at all when the request carries none, which is what lets a caller
    /// tell the two apart. Use
    /// [`locale_or_default`](Self::locale_or_default) or
    /// [`locale_with_default`](Self::locale_with_default) when any usable locale
    /// does.
    pub fn locale() -> Option<Locale> {
        LocaleContextHolder::locale_context().and_then(|context| context.locale())
    }

    /// Returns the locale of the request, or [`Locale::default`] when the code
    /// that runs has none.
    ///
    /// This is the convenient accessor for rendering a message: it never fails
    /// and always answers a usable locale.
    pub fn locale_or_default() -> Locale {
        Self::locale().unwrap_or_default()
    }

    /// Returns the locale of the request, or `default` when the code that runs
    /// has none.
    ///
    /// # Arguments
    ///
    /// * `default` - The locale to answer with outside a request.
    pub fn locale_with_default(default: Locale) -> Locale {
        Self::locale().unwrap_or(default)
    }
}
