use std::fmt;

use crate::Locale;

/// Error thrown when a message can't be resolved.
#[derive(Debug, Clone)]
pub struct NoSuchMessageError {
    pub code: String,
    pub locale: Option<Locale>,
}

impl NoSuchMessageError {
    pub fn new(code: &str, locale: Option<&Locale>) -> Self {
        NoSuchMessageError {
            code: code.to_string(),
            locale: locale.cloned(),
        }
    }
}

impl fmt::Display for NoSuchMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.locale {
            Some(locale) => write!(
                f,
                "No message found under code '{}' for locale '{}'.",
                self.code, locale
            ),
            None => write!(f, "No message found under code '{}'.", self.code),
        }
    }
}

impl std::error::Error for NoSuchMessageError {}
