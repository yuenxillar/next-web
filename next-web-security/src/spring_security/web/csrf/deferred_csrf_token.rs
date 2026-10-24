use std::sync::Arc;

use next_web_core::async_trait;

use crate::web::csrf::CsrfToken;

/// An interface that allows delayed access to a CsrfToken that may be generated.
#[async_trait]
pub trait DeferredCsrfToken
where
    Self: Send + Sync,
{
    /// Gets the CsrfToken
    async fn token(&mut self) -> Arc<dyn CsrfToken>;

    /// Returns true if get() refers to a generated CsrfToken or false if it already existed.
    async fn is_generated(&mut self) -> bool;
}
