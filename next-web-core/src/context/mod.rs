use crate::util::locale::Locale;

pub mod application_args;
pub mod application_context;

pub mod application_resources;
pub mod next_properties;
pub mod properties;

pub mod support;

pub trait MessageSource
where
    Self: Send + Sync,
{
    fn message(&self, code: &str, locale: Locale) -> Option<String>;

    fn message_with_args(&self, code: &str, args: &[&str], locale: Locale) -> Option<String>;

    fn message_or_default(&self, code: &str, locale: Locale) -> String;
}
