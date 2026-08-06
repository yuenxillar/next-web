use std::any::TypeId;

use crate::{ApplicationEvent, ApplicationListener};

pub trait SmartApplicationListener
where
    Self: ApplicationListener<Box<dyn ApplicationEvent>>,
{
    /// Determine whether this listener actually supports the given event type.
    fn supports_event_type(&self, event: TypeId) -> bool;

    /// Determine whether this listener actually supports the given event type.
    #[allow(unused_variables)]
    fn supports_source_type(&self, event: TypeId) -> bool {
        true
    }

    /// Determine this listener's order in a set of listeners for the same even
    fn order(&self) -> i32;

    fn listener_id(&self) -> &str {
        ""
    }
}
