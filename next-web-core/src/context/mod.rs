use crate::async_trait;
use crate::util::locale::Locale;

pub mod application_args;
pub mod application_context;

pub mod application_resources;
pub mod next_properties;
pub mod properties;

pub mod support;

#[async_trait]
pub trait MessageSource
where
    Self: Send + Sync,
{
    async fn message(&self, code: &str, locale: Locale) -> Option<String>;

    async fn message_with_args(&self, code: &str, args: &[&str], locale: Locale) -> Option<String>;

    async fn message_or_default(&self, code: &str, locale: Locale) -> String;
}
