use std::any::Any;

use crate::{ApplicationEvent, BoxFuture};

pub trait ApplicationListener<E>
where
    E: ApplicationEvent,
    Self: Any,
    Self: Send + Sync,
{
    /// Handle an application event.
    fn on_application_event<'a>(&'a self, event: E) -> BoxFuture<'a, ()>;
}
