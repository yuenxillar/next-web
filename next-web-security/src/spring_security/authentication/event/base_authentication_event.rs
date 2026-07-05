use std::sync::Arc;

use next_web_context::EventAttributes;

use crate::core::Authentication;

#[derive(Clone)]
pub struct BaseAuthenticationEvent {
    inner: EventAttributes<Arc<dyn Authentication>>,
}

impl BaseAuthenticationEvent {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            inner: EventAttributes::new(authentication),
        }
    }

    pub fn authentication(&self) -> &Arc<dyn Authentication> {
        self.inner
            .source()
            .downcast_ref::<Arc<dyn Authentication>>()
            .unwrap()
    }
}

impl AsRef<EventAttributes<Arc<dyn Authentication>>> for BaseAuthenticationEvent {
    fn as_ref(&self) -> &EventAttributes<Arc<dyn Authentication>> {
        &self.inner
    }
}
