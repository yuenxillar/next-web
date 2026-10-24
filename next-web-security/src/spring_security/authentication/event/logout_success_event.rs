use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::Arc,
};

use next_web_context::ApplicationEvent;

use crate::{authentication::event::BaseAuthenticationEvent, core::Authentication};

/// Application event which indicates successful logout
#[derive(Clone)]
pub struct LogoutSuccessEvent {
    base: BaseAuthenticationEvent,
}

impl LogoutSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            base: BaseAuthenticationEvent::new(authentication),
        }
    }
}

impl ApplicationEvent for LogoutSuccessEvent {
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

impl Deref for LogoutSuccessEvent {
    type Target = BaseAuthenticationEvent;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
