use std::sync::{Arc, RwLock};

use futures::future::BoxFuture;
use next_web_core::filter::FilterError;

use crate::core::context::{
    security_context::SecurityContext, SecurityContextHolderStrategy, SecurityContextImpl,
};

static CONTEXT_HOLDER: RwLock<Option<Arc<dyn SecurityContext>>> = RwLock::new(None);

/// A static field-based implementation of SecurityContextHolderStrategy.
/// All instances in the JVM share the same SecurityContext.
/// Useful for rich clients (desktop apps), not for web servers.
#[derive(Clone, Default)]
pub struct GlobalSecurityContextHolderStrategy;

impl SecurityContextHolderStrategy for GlobalSecurityContextHolderStrategy {
    fn clear_context(&self) {
        if let Ok(mut guard) = CONTEXT_HOLDER.write() {
            *guard = None;
        }
    }

    fn get_context(&self) -> Option<Arc<dyn SecurityContext>> {
        if let Some(ctx) = CONTEXT_HOLDER.read().ok().and_then(|g| g.clone()) {
            return Some(ctx);
        }

        let mut guard = CONTEXT_HOLDER.write().expect("lock poisoned");

        if guard.is_none() {
            *guard = Some(Arc::new(SecurityContextImpl::default()));
        }

        guard.clone()
    }

    fn set_context(&self, context: Arc<dyn SecurityContext>) {
        if let Ok(mut guard) = CONTEXT_HOLDER.write() {
            *guard = Some(context);
        }
    }

    fn scope_with_context<'a>(
        &'a self,
        context: Arc<dyn SecurityContext>,
        f: BoxFuture<'a, Result<(), FilterError>>,
    ) -> BoxFuture<'a, Result<(), FilterError>> {
        todo!()
    }

    fn create_empty_context(&self) -> Arc<dyn SecurityContext> {
        Arc::new(SecurityContextImpl::default())
    }
}
