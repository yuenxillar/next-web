use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::core::context::thread_local_security_context_holder_strategy::ThreadLocalSecurityContextHolderStrategy;
use crate::core::context::{
    security_context_changed_event::SecurityContextChangedEvent,
    security_context_changed_listener::SecurityContextChangedListener, SecurityContext,
    SecurityContextHolderStrategy, SecurityContextSupplier,
};

/// Publishes an event when its delegate's context is replaced or cleared.
#[derive(Clone)]
pub struct ListeningSecurityContextHolderStrategy {
    delegate: Arc<dyn SecurityContextHolderStrategy>,
    listeners: Arc<[Arc<dyn SecurityContextChangedListener>]>,
}

impl ListeningSecurityContextHolderStrategy {
    pub fn with_listeners(listeners: Vec<Arc<dyn SecurityContextChangedListener>>) -> Self {
        Self::new(
            Arc::new(ThreadLocalSecurityContextHolderStrategy),
            listeners,
        )
    }

    pub fn new(
        delegate: Arc<dyn SecurityContextHolderStrategy>,
        listeners: Vec<Arc<dyn SecurityContextChangedListener>>,
    ) -> Self {
        assert!(!listeners.is_empty(), "listeners cannot be empty");
        Self {
            delegate,
            listeners: listeners.into(),
        }
    }

    fn publish(
        listeners: &[Arc<dyn SecurityContextChangedListener>],
        event: SecurityContextChangedEvent,
    ) {
        for listener in listeners {
            listener.security_context_changed(event.clone());
        }
    }
}

impl SecurityContextHolderStrategy for ListeningSecurityContextHolderStrategy {
    fn clear_context(&self) {
        let old = self.delegate.get_deferred_context();
        self.delegate.clear_context();
        let old = Arc::new(move || Some(old()));
        Self::publish(&self.listeners, SecurityContextChangedEvent::cleared(old));
    }

    fn get_context(&self) -> Arc<dyn SecurityContext> {
        self.delegate.get_context()
    }

    fn get_deferred_context(&self) -> SecurityContextSupplier {
        self.delegate.get_deferred_context()
    }

    fn set_context(&self, context: Arc<dyn SecurityContext>) {
        self.set_deferred_context(Arc::new(move || context.clone()));
    }

    fn set_deferred_context(&self, updated: SecurityContextSupplier) {
        let old = self.delegate.get_deferred_context();
        let listeners = self.listeners.clone();
        let published = Arc::new(AtomicBool::new(false));
        let supplier = Arc::new(move || {
            let updated_context = updated();
            if published
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                let old_context = old();
                if !Arc::ptr_eq(&old_context, &updated_context) {
                    Self::publish(
                        &listeners,
                        SecurityContextChangedEvent::new(
                            old_context,
                            Some(updated_context.clone()),
                        ),
                    );
                }
            }
            updated_context
        });
        self.delegate.set_deferred_context(supplier);
    }

    fn create_empty_context(&self) -> Arc<dyn SecurityContext> {
        self.delegate.create_empty_context()
    }
}
