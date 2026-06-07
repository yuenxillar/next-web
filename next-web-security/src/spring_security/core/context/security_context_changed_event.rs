use std::sync::Arc;

use crate::core::context::security_context::SecurityContext;

type ContextSupplier = Arc<dyn Fn() -> Option<Arc<dyn SecurityContext>> + Send + Sync>;

/// Event that represents a change in SecurityContext.
#[derive(Clone)]
pub struct SecurityContextChangedEvent {
    old_context: ContextSupplier,
    new_context: ContextSupplier,
}

impl SecurityContextChangedEvent {
    /// Sentinel indicating the context was cleared (no new context).
    pub fn no_context() -> ContextSupplier {
        Arc::new(|| None)
    }

    /// Construct from existing contexts.
    pub fn new(
        old_context: Arc<dyn SecurityContext>,
        new_context: Option<Arc<dyn SecurityContext>>,
    ) -> Self {
        let old: ContextSupplier = Arc::new(move || Some(old_context.clone()));
        let new: ContextSupplier = match new_context {
            Some(ctx) => Arc::new(move || Some(ctx.clone())),
            None => Self::no_context(),
        };
        Self {
            old_context: old,
            new_context: new,
        }
    }

    /// Construct from supplier functions.
    pub fn from_suppliers(old_context: ContextSupplier, new_context: ContextSupplier) -> Self {
        Self {
            old_context,
            new_context,
        }
    }

    /// Get the previous SecurityContext.
    pub fn get_old_context(&self) -> Option<Arc<dyn SecurityContext>> {
        (self.old_context)()
    }

    /// Get the current/new SecurityContext.
    pub fn get_new_context(&self) -> Option<Arc<dyn SecurityContext>> {
        (self.new_context)()
    }

    /// Whether this event represents clearing the context.
    pub fn is_cleared(&self) -> bool {
        self.get_new_context().is_none()
    }
}
