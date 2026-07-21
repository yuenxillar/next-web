use std::sync::{Arc, RwLock};

use futures::future::BoxFuture;
use next_web_core::filter::FilterError;

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::SecurityContextHolderStrategy,
};

static GLOBAL_CONTEXT: RwLock<Option<Arc<dyn SecurityContext>>> = RwLock::new(None);

/// A static field-based implementation of SecurityContextHolderStrategy.
/// All instances in the JVM share the same SecurityContext.
/// Useful for rich clients (desktop apps), not for web servers.
#[derive(Clone, Default)]
pub struct GlobalSecurityContextHolderStrategy;

impl SecurityContextHolderStrategy for GlobalSecurityContextHolderStrategy {
    fn clear_context(&self) {
        if let Ok(mut guard) = GLOBAL_CONTEXT.write() {
            *guard = None;
        }
    }

    fn get_context(&self) -> Option<Arc<dyn SecurityContext>> {
        // let guard = GLOBAL_CONTEXT.read().expect("global context lock poisoned");
        // if let Some(ref ctx) = *guard {
        //     Some(ctx.clone())
        // } else {
        //     drop(guard);
        //     let mut write_guard = GLOBAL_CONTEXT
        //         .write()
        //         .expect("global context lock poisoned");
        //     if write_guard.is_none() {
        //         *write_guard = Some(SecurityContext::default());
        //     }
        //     write_guard.clone()
        // }
        todo!()
    }

    fn set_context(&self, context: Arc<dyn SecurityContext>) {
        if let Ok(mut guard) = GLOBAL_CONTEXT.write() {
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
        todo!()
    }
}
