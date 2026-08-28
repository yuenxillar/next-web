use std::{any::Any, sync::Arc};

use next_web_context::{ApplicationEvent, EventAttributes};
use next_web_core::BoxAny;

use crate::{authorization::authorization_result::AuthorizationResult, core::Authentication};

/// Base event for authorization results.
pub struct AuthorizationEvent {
    authentication: Arc<dyn Authentication>,
    result: Box<dyn AuthorizationResult>,

    base: EventAttributes<BoxAny>,
}

impl AuthorizationEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        source: BoxAny,

        result: Box<dyn AuthorizationResult>,
    ) -> Self {
        Self {
            authentication,
            result,

            inner: EventAttributes::new(source),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }

    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.result.as_ref()
    }
}

impl ApplicationEvent for AuthorizationEvent {
    fn timestamp(&self) -> u64 {
        self.base.timestamp()
    }

    fn source(&self) -> &dyn Any {
        self.base.source()
    }
}
