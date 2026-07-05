use std::{any::Any, ops::Deref, sync::Arc};

use next_web_context::ApplicationEvent;

use crate::{authentication::event::BaseAuthenticationEvent, core::Authentication};

/// Application event which indicates successful logout
#[derive(Clone)]
pub struct LogoutSuccessEvent {
    inner: BaseAuthenticationEvent,
}

impl LogoutSuccessEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            inner: BaseAuthenticationEvent::new(authentication),
        }
    }
}

impl ApplicationEvent for LogoutSuccessEvent {
    fn timestamp(&self) -> u64 {
        self.as_ref().timestamp()
    }

    fn source(&self) -> &dyn Any {
        self.as_ref().source()
    }
}

impl Deref for LogoutSuccessEvent {
    type Target = BaseAuthenticationEvent;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
