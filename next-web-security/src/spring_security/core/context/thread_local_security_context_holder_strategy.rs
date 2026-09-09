use std::{cell::RefCell, sync::Arc};

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::{SecurityContextHolderStrategy, SecurityContextSupplier},
    SecurityContextImpl,
};

thread_local! {
    static CONTEXT_HOLDER: RefCell<Option<SecurityContextSupplier>> = RefCell::new(None);
}

/// A thread-local implementation of SecurityContextHolderStrategy.
#[derive(Clone, Default)]
pub struct ThreadLocalSecurityContextHolderStrategy;

impl ThreadLocalSecurityContextHolderStrategy {
    fn current_or_empty() -> Arc<dyn SecurityContext> {
        Self::current_supplier()()
    }

    fn current_supplier() -> SecurityContextSupplier {
        CONTEXT_HOLDER.with(|cell| match cell.borrow().as_ref().cloned() {
            Some(supplier) => supplier,
            None => {
                let context = Arc::new(SecurityContextImpl::default()) as Arc<dyn SecurityContext>;
                let supplier: SecurityContextSupplier = Arc::new(move || context.clone());
                *cell.borrow_mut() = Some(supplier.clone());
                supplier
            }
        })
    }
}

impl SecurityContextHolderStrategy for ThreadLocalSecurityContextHolderStrategy {
    fn clear_context(&self) {
        CONTEXT_HOLDER.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }

    fn get_context(&self) -> Arc<dyn SecurityContext> {
        Self::current_or_empty()
    }

    fn get_deferred_context(&self) -> SecurityContextSupplier {
        Self::current_supplier()
    }

    fn set_context(&self, context: Arc<dyn SecurityContext>) {
        self.set_deferred_context(Arc::new(move || context.clone()));
    }

    fn set_deferred_context(&self, deferred_context: SecurityContextSupplier) {
        CONTEXT_HOLDER.with(|cell| {
            *cell.borrow_mut() = Some(deferred_context);
        });
    }

    fn create_empty_context(&self) -> Arc<dyn SecurityContext> {
        Arc::new(SecurityContextImpl::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn context_is_reused_until_cleared() {
        let strategy = ThreadLocalSecurityContextHolderStrategy;
        strategy.clear_context();
        let first = strategy.get_context();
        let second = strategy.get_context();
        assert!(Arc::ptr_eq(&first, &second));
        strategy.clear_context();
        let third = strategy.get_context();
        assert!(!Arc::ptr_eq(&first, &third));
    }

    #[test]
    fn deferred_context_is_evaluated_on_get() {
        let strategy = ThreadLocalSecurityContextHolderStrategy;
        strategy.clear_context();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_supplier = calls.clone();
        let context = strategy.create_empty_context();
        strategy.set_deferred_context(Arc::new(move || {
            calls_for_supplier.fetch_add(1, Ordering::Relaxed);
            context.clone()
        }));
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        let _ = strategy.get_context();
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
