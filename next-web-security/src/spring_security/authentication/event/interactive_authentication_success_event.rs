use std::{any::TypeId, sync::Arc};

use next_web_context::ApplicationEvent;

use crate::{authentication::event::BaseAuthenticationEvent, core::Authentication};

#[derive(Clone)]
pub struct InteractiveAuthenticationSuccessEvent {
    generated_by: TypeId,

    inner: BaseAuthenticationEvent,
}

impl InteractiveAuthenticationSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>, generated_by: TypeId) -> Self {
        Self {
            generated_by,
            inner: BaseAuthenticationEvent::new(authentication),
        }
    }

    /// Getter for the TypeId that generated this event. This can be useful for generating additional logging information.
    pub fn get_generated_by(&self) -> TypeId {
        self.generated_by
    }
}

impl ApplicationEvent for InteractiveAuthenticationSuccessEvent {
    fn source(&self) -> &dyn std::any::Any {
        self.inner.source()
    }

    fn timestamp(&self) -> u64 {
        self.inner.timestamp()
    }
}
