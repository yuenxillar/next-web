use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_context::{ApplicationEvent, EventAttributes};
use next_web_core::BoxAny;

use crate::{authorization::authorization_result::AuthorizationResult, core::Authentication};

/// Base event for authorization results.
pub struct AuthorizationEvent {
    authentication: Arc<dyn Authentication>,
    result: Arc<dyn AuthorizationResult>,
    source: Arc<dyn Any + Send + Sync>,

    base: EventAttributes<Arc<dyn Any + Send + Sync>>,
}

impl AuthorizationEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        source: BoxAny,

        result: Arc<dyn AuthorizationResult>,
    ) -> Self {
        let source: Arc<dyn Any + Send + Sync> = Arc::from(source);
        Self {
            authentication,
            result,
            source: source.clone(),
            base: EventAttributes::new(source),
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }

    pub fn authorization_result(&self) -> &dyn AuthorizationResult {
        self.result.as_ref()
    }

    pub fn object(&self) -> &dyn Any {
        self.base.source()
    }
}

impl Clone for AuthorizationEvent {
    fn clone(&self) -> Self {
        Self {
            authentication: self.authentication.clone(),
            result: self.result.clone(),
            source: self.source.clone(),
            base: EventAttributes::new(self.source.clone()),
        }
    }
}

impl ApplicationEvent for AuthorizationEvent {
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
