use std::sync::Arc;

use crate::core::context::{
    DeferredSecurityContext, SecurityContext, SecurityContextHolderStrategy,
};

/// A DeferredSecurityContext backed by a pre-existing context.
#[derive(Clone)]
pub struct SuppliedDeferredSecurityContext {
    strategy: Arc<dyn SecurityContextHolderStrategy>,

    security_context: Option<Arc<dyn SecurityContext>>,
}

impl SuppliedDeferredSecurityContext {
    pub fn new(
        security_context: Option<Arc<dyn SecurityContext>>,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) -> Self {
        Self {
            strategy,
            security_context,
        }
    }

    fn init(&mut self) {
        if self.security_context.is_some() {
            return;
        }

        self.security_context = Some(self.strategy.create_empty_context());
    }
}

impl DeferredSecurityContext for SuppliedDeferredSecurityContext {
    fn get(&mut self) -> Option<Arc<dyn SecurityContext>> {
        self.init();

        self.security_context.clone()
    }

    fn is_generated(&mut self) -> bool {
        self.init();

        self.security_context.is_none()
    }
}
