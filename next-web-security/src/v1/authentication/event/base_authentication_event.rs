use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

use next_web_context::{ApplicationEvent, EventAttributes};

use crate::core::Authentication;

#[derive(Clone)]
pub struct BaseAuthenticationEvent {
    base: EventAttributes<Arc<dyn Authentication>>,
}

impl BaseAuthenticationEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            base: EventAttributes::new(authentication),
        }
    }

    pub fn authentication(&self) -> &Arc<dyn Authentication> {
        self.base.value()
    }
}

impl ApplicationEvent for BaseAuthenticationEvent {
    fn timestamp(&self) -> u64 {
        self.base.timestamp()
    }

    fn source(&self) -> &dyn Any {
        self.base.source()
    }

    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn source_type(&self) -> TypeId {
        self.base.source_type()
    }
}

impl Deref for BaseAuthenticationEvent {
    type Target = EventAttributes<Arc<dyn Authentication>>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
