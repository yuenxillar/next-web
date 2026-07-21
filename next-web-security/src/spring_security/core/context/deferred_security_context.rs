use std::sync::Arc;

use crate::core::context::security_context::SecurityContext;

/// Allows delayed access to a SecurityContext that may be generated.
pub trait DeferredSecurityContext
where
    Self: Send + Sync,
{
    /// Returns the SecurityContext, generating it if necessary.
    fn get(&mut self) -> Option<Arc<dyn SecurityContext>>;

    /// Returns true if `get()` refers to a generated SecurityContext,
    /// or false if it already existed.
    fn is_generated(&mut self) -> bool;
}

// A DeferredSecurityContext that lazily generates the context.
// pub struct GeneratedDeferredSecurityContext {
//     context: Arc<dyn SecurityContext>,
// }

// impl GeneratedDeferredSecurityContext {
//     pub fn new(context: Arc<dyn SecurityContext>) -> Self {
//         Self { context }
//     }
// }

// impl DeferredSecurityContext for GeneratedDeferredSecurityContext {
//     fn get(&self) -> Option<Arc<dyn SecurityContext>> {
//         self.context.clone()
//     }

//     fn is_generated(&self) -> bool {
//         true
//     }
// }
