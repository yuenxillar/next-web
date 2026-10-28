use std::sync::Arc;

use crate::core::context::security_context::SecurityContext;

/// A lazily evaluated security context supplier.
pub type SecurityContextSupplier = Arc<dyn Fn() -> Arc<dyn SecurityContext> + Send + Sync>;

/// A strategy for storing security context information against an execution scope.
/// The preferred strategy is loaded by SecurityContextHolder.
pub trait SecurityContextHolderStrategy
where
    Self: Send + Sync,
{
    /// Clears the current context.
    fn clear_context(&self);

    /// Obtains the current context.
    fn get_context(&self) -> Arc<dyn SecurityContext>;

    /// Obtains a supplier for the current context without eagerly evaluating it.
    fn get_deferred_context(&self) -> SecurityContextSupplier {
        let context = self.get_context();
        Arc::new(move || context.clone())
    }

    /// Sets the current context.
    fn set_context(&self, context: Arc<dyn SecurityContext>);

    /// Installs a supplier and evaluates it only when the context is requested.
    fn set_deferred_context(&self, deferred_context: SecurityContextSupplier) {
        let context = deferred_context();
        self.set_context(context);
    }

    /// Creates a new, empty context implementation.
    fn create_empty_context(&self) -> Arc<dyn SecurityContext>;
}
