use std::sync::RwLock;

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::SecurityContextHolderStrategy,
};

static GLOBAL_CONTEXT: RwLock<Option<SecurityContext>> = RwLock::new(None);

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

    fn get_context(&self) -> SecurityContext {
        let guard = GLOBAL_CONTEXT.read().expect("global context lock poisoned");
        if let Some(ref ctx) = *guard {
            ctx.clone()
        } else {
            drop(guard);
            let mut write_guard = GLOBAL_CONTEXT.write().expect("global context lock poisoned");
            if write_guard.is_none() {
                *write_guard = Some(SecurityContext::default());
            }
            write_guard.clone().unwrap()
        }
    }

    fn set_context(&self, context: SecurityContext) {
        if let Ok(mut guard) = GLOBAL_CONTEXT.write() {
            *guard = Some(context);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::context::{
        global_security_context_holder_strategy::GlobalSecurityContextHolderStrategy,
        security_context::SecurityContext,
        security_context_holder_strategy::SecurityContextHolderStrategy,
    };

    #[test]
    fn global_strategy_stores_and_retrieves_context() {
        let strategy = GlobalSecurityContextHolderStrategy;
        strategy.clear_context();

        let ctx = strategy.get_context();
        assert!(ctx.get_authentication().is_none());

        strategy.set_context(SecurityContext::default());
        let ctx = strategy.get_context();
        assert!(ctx.get_authentication().is_none());

        strategy.clear_context();
    }
}
