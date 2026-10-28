use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, OnceLock, RwLock,
};

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::{SecurityContextHolderStrategy, SecurityContextSupplier},
    thread_local_security_context_holder_strategy::ThreadLocalSecurityContextHolderStrategy,
};

type Strategy = Arc<dyn SecurityContextHolderStrategy>;

static STRATEGY: OnceLock<RwLock<Strategy>> = OnceLock::new();
static INITIALIZE_COUNT: AtomicUsize = AtomicUsize::new(1);

pub struct SecurityContextHolder;

impl SecurityContextHolder {
    pub const MODE_THREADLOCAL: &'static str = "MODE_THREADLOCAL";
    pub const MODE_GLOBAL: &'static str = "MODE_GLOBAL";

    fn strategy() -> &'static RwLock<Strategy> {
        STRATEGY.get_or_init(|| {
            RwLock::new(Arc::new(ThreadLocalSecurityContextHolderStrategy) as Arc<_>)
        })
    }

    pub fn clear_context() {
        Self::get_context_holder_strategy().clear_context();
    }

    pub fn get_context() -> Arc<dyn SecurityContext> {
        Self::get_context_holder_strategy().get_context()
    }

    pub fn set_context(context: Arc<dyn SecurityContext>) {
        Self::get_context_holder_strategy().set_context(context);
    }

    pub fn get_deferred_context() -> SecurityContextSupplier {
        Self::get_context_holder_strategy().get_deferred_context()
    }

    pub fn set_deferred_context(deferred_context: SecurityContextSupplier) {
        Self::get_context_holder_strategy().set_deferred_context(deferred_context);
    }

    pub fn set_context_holder_strategy(strategy: Strategy) {
        let mut guard = match Self::strategy().write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = strategy;
        INITIALIZE_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_context_holder_strategy() -> Strategy {
        match Self::strategy().read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn get_initialize_count() -> usize {
        INITIALIZE_COUNT.load(Ordering::Relaxed)
    }

    pub fn create_empty_context() -> Arc<dyn SecurityContext> {
        Self::get_context_holder_strategy().create_empty_context()
    }
}
