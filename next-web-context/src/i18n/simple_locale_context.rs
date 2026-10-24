use crate::{Locale, i18n::LocaleContext};

/// A [`LocaleContext`] that always returns the locale it was created with.
///
/// The type is the counterpart of `SimpleLocaleContext`, and it is the context
/// [`LocaleContextHolder::set_locale`](crate::i18n::LocaleContextHolder::set_locale)
/// creates for a locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleLocaleContext {
    locale: Locale,
}

impl SimpleLocaleContext {
    /// Creates a context returning the given locale.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale the context returns.
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    /// Returns the locale of this context.
    pub fn locale_value(&self) -> &Locale {
        &self.locale
    }
}

impl LocaleContext for SimpleLocaleContext {
    fn locale(&self) -> Option<Locale> {
        Some(self.locale.clone())
    }
}

impl From<Locale> for SimpleLocaleContext {
    fn from(locale: Locale) -> Self {
        Self::new(locale)
    }
}
