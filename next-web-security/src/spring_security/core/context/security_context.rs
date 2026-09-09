use std::sync::Arc;

use crate::core::Authentication;

/// Trait defining the minimum security information associated with the current thread of execution.
/// The security context is stored in a SecurityContextHolder.
pub trait SecurityContext
where
    Self: Send + Sync,
{
    /// Obtains the currently authenticated principal, or an authentication request token.
    fn get_authentication(&self) -> Option<Arc<dyn Authentication>>;

    /// Changes the currently authenticated principal, or removes the authentication information.
    fn set_authentication(&self, authentication: Option<Arc<dyn Authentication>>);
}
