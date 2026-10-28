use std::sync::Arc;

use crate::core::{context::security_context::SecurityContext, Authentication};

use super::SecurityContextImpl;

/// A SecurityContext that should never be stored across requests.
/// Useful when running as a different user for part of a request.
#[derive(Clone, Default)]
pub struct TransientSecurityContext {
    inner: SecurityContextImpl,
}

impl TransientSecurityContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_authentication(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            inner: SecurityContextImpl::new(authentication),
        }
    }

    pub fn get_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.inner.get_authentication()
    }

    pub fn set_authentication(&self, authentication: Option<Arc<dyn Authentication>>) {
        self.inner.set_authentication(authentication);
    }
}

impl SecurityContext for TransientSecurityContext {
    fn get_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.inner.get_authentication()
    }

    fn set_authentication(&self, authentication: Option<Arc<dyn Authentication>>) {
        self.inner.set_authentication(authentication);
    }
}
