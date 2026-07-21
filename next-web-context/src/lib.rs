#![allow(missing_docs)]

pub mod support;

mod application_context;
mod application_event;
mod application_event_publisher;
mod message_source;
mod message_source_resolvable;
mod no_such_message_error;

pub use application_context::ApplicationContext;
pub use application_event::{ApplicationEvent, EventAttributes};
pub use application_event_publisher::ApplicationEventPublisher;
pub use message_source::MessageSource;
pub use message_source_resolvable::MessageSourceResolvable;
pub use no_such_message_error::NoSuchMessageError;

#[derive(Debug, Clone)]
pub struct Locale;

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "en")
    }
}
