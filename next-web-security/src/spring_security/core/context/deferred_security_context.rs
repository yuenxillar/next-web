use crate::core::context::security_context::SecurityContext;

/// Allows delayed access to a SecurityContext that may be generated.
pub trait DeferredSecurityContext: Send {
    /// Returns the SecurityContext, generating it if necessary.
    fn get(&self) -> SecurityContext;

    /// Returns true if `get()` refers to a generated SecurityContext,
    /// or false if it already existed.
    fn is_generated(&self) -> bool;
}

/// A DeferredSecurityContext backed by a pre-existing context.
pub struct SuppliedDeferredSecurityContext {
    context: SecurityContext,
}

impl SuppliedDeferredSecurityContext {
    pub fn new(context: SecurityContext) -> Self {
        Self { context }
    }
}

impl DeferredSecurityContext for SuppliedDeferredSecurityContext {
    fn get(&self) -> SecurityContext {
        self.context.clone()
    }

    fn is_generated(&self) -> bool {
        false
    }
}

/// A DeferredSecurityContext that lazily generates the context.
pub struct GeneratedDeferredSecurityContext {
    context: SecurityContext,
}

impl GeneratedDeferredSecurityContext {
    pub fn new(context: SecurityContext) -> Self {
        Self { context }
    }
}

impl DeferredSecurityContext for GeneratedDeferredSecurityContext {
    fn get(&self) -> SecurityContext {
        self.context.clone()
    }

    fn is_generated(&self) -> bool {
        true
    }
}
