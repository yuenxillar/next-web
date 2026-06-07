use std::sync::{Arc, OnceLock};

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::SecurityContextHolderStrategy,
    thread_local_security_context_holder_strategy::ThreadLocalSecurityContextHolderStrategy,
};

type Strategy = Arc<dyn SecurityContextHolderStrategy>;

static STRATEGY: OnceLock<Strategy> = OnceLock::new();

pub struct SecurityContextHolder;

impl SecurityContextHolder {
    pub fn clear_context() {
        Self::get_context_holder_strategy().clear_context();
    }

    pub fn get_context() -> Option<Arc<dyn SecurityContext>> {
        Self::get_context_holder_strategy().get_context()
    }

    pub fn set_context(context: Arc<dyn SecurityContext>) {
        Self::get_context_holder_strategy().set_context(context);
    }

    // pub fn set_context_holder_strategy(strategy: Arc<dyn SecurityContextHolderStrategy>) {
    //     let _ = STRATEGY.set(strategy);
    // }

    pub fn get_context_holder_strategy() -> Strategy {
        STRATEGY
            .get_or_init(|| Arc::new(ThreadLocalSecurityContextHolderStrategy::default()) as Arc<_>)
            .clone()
    }

    pub fn create_empty_context() -> Arc<dyn SecurityContext> {
        Self::get_context_holder_strategy().create_empty_context()
    }
}
