//! Port of `org.springframework.context.i18n.LocaleContext`.

use std::fmt;

use crate::Locale;

/// Strategy interface for determining the current locale.
///
/// A locale context can be associated with the code that is currently running
/// through [`LocaleContextHolder`](crate::i18n::LocaleContextHolder).
///
/// The locale a context returns can be fixed, or it can be determined
/// dynamically, depending on the strategy the implementation uses.
pub trait LocaleContext
where
    Self: fmt::Debug,
    Self: Send + Sync,
{
    /// Returns the current locale.
    ///
    /// Equivalent to `getLocale()`.
    ///
    /// # Returns
    ///
    /// The locale of this context, or `None` when no specific locale is
    /// associated with it.
    fn locale(&self) -> Option<Locale>;
}
