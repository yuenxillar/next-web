use std::sync::{Arc, RwLock};

use crate::core::context::{
    security_context::SecurityContext, SecurityContextHolderStrategy, SecurityContextImpl,
};

static CONTEXT_HOLDER: RwLock<Option<Arc<dyn SecurityContext>>> = RwLock::new(None);

/// A static field-based implementation of SecurityContextHolderStrategy.
/// All instances share the same SecurityContext.
#[derive(Clone, Default)]
pub struct GlobalSecurityContextHolderStrategy;

impl GlobalSecurityContextHolderStrategy {
    fn current_context() -> Arc<dyn SecurityContext> {
        let read_guard = match CONTEXT_HOLDER.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(context) = read_guard.clone() {
            return context;
        }
        drop(read_guard);

        let mut guard = match CONTEXT_HOLDER.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_none() {
            *guard = Some(Arc::new(SecurityContextImpl::default()));
        }

        guard
            .clone()
            .unwrap_or_else(|| Arc::new(SecurityContextImpl::default()))
    }
}

impl SecurityContextHolderStrategy for GlobalSecurityContextHolderStrategy {
    fn clear_context(&self) {
        let mut guard = match CONTEXT_HOLDER.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = None;
    }

    fn get_context(&self) -> Arc<dyn SecurityContext> {
        Self::current_context()
    }

    fn set_context(&self, context: Arc<dyn SecurityContext>) {
        let mut guard = match CONTEXT_HOLDER.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = Some(context);
    }

    fn create_empty_context(&self) -> Arc<dyn SecurityContext> {
        Arc::new(SecurityContextImpl::default())
    }
}
