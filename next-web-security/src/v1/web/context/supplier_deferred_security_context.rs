use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};

use crate::core::context::{
    DeferredSecurityContext, SecurityContext, SecurityContextHolderStrategy,
};

/// A DeferredSecurityContext backed by a pre-existing context.
#[derive(Clone)]
pub struct SuppliedDeferredSecurityContext {
    strategy: Arc<dyn SecurityContextHolderStrategy>,

    security_context: Arc<OnceLock<Arc<dyn SecurityContext>>>,
    generated: Arc<AtomicBool>,
}

impl SuppliedDeferredSecurityContext {
    pub fn new(
        security_context: Option<Arc<dyn SecurityContext>>,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) -> Self {
        let holder = OnceLock::new();
        if let Some(context) = security_context {
            let _ = holder.set(context);
        }
        Self {
            strategy,
            security_context: Arc::new(holder),
            generated: Arc::new(AtomicBool::new(false)),
        }
    }

    fn init(&self) -> Arc<dyn SecurityContext> {
        self.security_context
            .get_or_init(|| {
                self.generated.store(true, Ordering::Release);
                self.strategy.create_empty_context()
            })
            .clone()
    }
}

impl DeferredSecurityContext for SuppliedDeferredSecurityContext {
    fn get(&self) -> Arc<dyn SecurityContext> {
        self.init()
    }

    fn is_generated(&self) -> bool {
        self.init();
        self.generated.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::thread_local_security_context_holder_strategy::ThreadLocalSecurityContextHolderStrategy;

    #[test]
    fn generated_context_is_cached_and_reported() {
        let strategy: Arc<dyn SecurityContextHolderStrategy> =
            Arc::new(ThreadLocalSecurityContextHolderStrategy);
        let deferred = SuppliedDeferredSecurityContext::new(None, strategy);
        assert!(deferred.is_generated());
        let first = deferred.get();
        let second = deferred.get();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(deferred.is_generated());
    }
}
