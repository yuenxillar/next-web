use std::{
    fmt::{self},
    sync::Arc,
};

use crate::{Locale, MessageSource};

/// Helper class for easy access to messages from a MessageSource, providing various overloaded getMessage methods.
/// Available from ApplicationObjectSupport, but also reusable as a standalone helper to delegate to in
/// application objects.
#[derive(Clone)]
pub struct MessageSourceAccessor {
    message_source: Arc<dyn MessageSource>,
    default_locale: Option<Locale>,
}

impl MessageSourceAccessor {
    /// Create a new MessageSourceAccessor, using LocaleContextHolder's locale as default locale.
    pub fn new(message_source: Arc<dyn MessageSource>) -> Self {
        Self {
            message_source,
            default_locale: None,
        }
    }

    /// Create a new MessageSourceAccessor, using the given default locale.
    pub fn with_default_locale(
        message_source: Arc<dyn MessageSource>,
        default_locale: Locale,
    ) -> Self {
        Self {
            message_source,
            default_locale: Some(default_locale),
        }
    }

    /// Return the default locale to use if no explicit locale has been given.
    /// The default implementation returns the default locale passed into the corresponding constructor,
    /// or LocaleContextHolder's locale as fallback. Can be overridden in subclasses.
    pub fn default_locale(&self) -> Option<&Locale> {
        self.default_locale.as_ref()
    }

    pub fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default: &str,
    ) -> String {
        todo!()
    }
}
