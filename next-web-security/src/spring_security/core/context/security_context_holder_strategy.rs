use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::filter::FilterError;

use crate::core::context::security_context::SecurityContext;

/// A strategy for storing security context information against a thread.
/// The preferred strategy is loaded by SecurityContextHolder.
pub trait SecurityContextHolderStrategy
where
    Self: Send + Sync,
{
    /// Clears the current context.
    fn clear_context(&self);

    /// Obtains the current context.
    fn get_context(&self) -> Option<Arc<dyn SecurityContext>>;

    /// Sets the current context.
    fn set_context(&self, context: Arc<dyn SecurityContext>);

    fn scope_with_context<'a>(
        &'a self,
        context: Arc<dyn SecurityContext>,
        func: BoxFuture<'a, Result<(), FilterError>>,
    ) -> BoxFuture<'a, Result<(), FilterError>>;

    /// Creates a new, empty context implementation, for use by SecurityContextRepository
    /// implementations, when creating a new context for the first time
    fn create_empty_context(&self) -> Arc<dyn SecurityContext>;
}
