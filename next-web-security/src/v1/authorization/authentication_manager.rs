use next_web_core::async_trait;
use std::{any::Any, sync::Arc};

use crate::core::{Authentication, AuthenticationError};

#[async_trait]
pub trait AuthenticationManager
where
    Self: Send + Sync,
    Self: Any,
{
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError>;
}
