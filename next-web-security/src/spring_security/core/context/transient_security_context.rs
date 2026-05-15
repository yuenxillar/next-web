use std::sync::Arc;

use crate::core::{
    authentication::Authentication,
    context::security_context::SecurityContext,
};

/// A SecurityContext that should never be stored across requests.
/// Useful when running as a different user for part of a request.
#[derive(Clone)]
pub struct TransientSecurityContext {
    inner: SecurityContext,
}

impl TransientSecurityContext {
    pub fn new() -> Self {
        Self {
            inner: SecurityContext::default(),
        }
    }

    pub fn with_authentication(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            inner: SecurityContext::new(Some(authentication)),
        }
    }

    pub fn get_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.inner.get_authentication()
    }

    pub fn set_authentication(&mut self, authentication: Option<Arc<dyn Authentication>>) {
        self.inner.set_authentication(authentication);
    }
}

impl Default for TransientSecurityContext {
    fn default() -> Self {
        Self::new()
    }
}
