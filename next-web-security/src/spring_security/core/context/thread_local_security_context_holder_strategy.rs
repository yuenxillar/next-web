use std::cell::RefCell;

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::SecurityContextHolderStrategy,
};

thread_local! {
    static THREAD_LOCAL_CONTEXT: RefCell<SecurityContext> = RefCell::new(SecurityContext::default());
}

#[derive(Clone, Default)]
pub struct ThreadLocalSecurityContextHolderStrategy;

impl SecurityContextHolderStrategy for ThreadLocalSecurityContextHolderStrategy {
    fn clear_context(&self) {
        THREAD_LOCAL_CONTEXT.with(|context| {
            *context.borrow_mut() = SecurityContext::default();
        });
    }

    fn get_context(&self) -> SecurityContext {
        THREAD_LOCAL_CONTEXT.with(|context| context.borrow().clone())
    }

    fn set_context(&self, context: SecurityContext) {
        THREAD_LOCAL_CONTEXT.with(|slot| {
            *slot.borrow_mut() = context;
        });
    }
}
