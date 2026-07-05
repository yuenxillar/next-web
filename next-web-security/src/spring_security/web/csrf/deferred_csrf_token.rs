use std::sync::Arc;

use crate::web::csrf::CsrfToken;

pub trait DeferredCsrfToken
where
    Self: Send + Sync,
{
    fn token(&self) -> Arc<dyn CsrfToken>;

    fn is_generated(&self) -> bool;
}
