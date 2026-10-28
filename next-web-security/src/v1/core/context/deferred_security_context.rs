use std::sync::Arc;

use crate::core::context::security_context::SecurityContext;

/// Allows delayed access to a SecurityContext that may be generated.
pub trait DeferredSecurityContext
where
    Self: Send + Sync,
{
    /// Returns the SecurityContext, generating it if necessary.
    fn get(&self) -> Arc<dyn SecurityContext>;

    /// Returns true if `get()` refers to a generated SecurityContext,
    /// or false if it already existed.
    fn is_generated(&self) -> bool;
}
