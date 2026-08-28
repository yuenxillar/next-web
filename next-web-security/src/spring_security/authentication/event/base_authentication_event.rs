use std::{ops::Deref, sync::Arc};

use next_web_context::EventAttributes;

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
        self.base
            .source()
            .downcast_ref::<Arc<dyn Authentication>>()
            .unwrap()
    }
}

impl Deref for BaseAuthenticationEvent {
    type Target = EventAttributes<Arc<dyn Authentication>>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
