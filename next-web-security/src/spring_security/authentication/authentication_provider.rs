use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{Authentication, authentication_error::AuthenticationError};

#[async_trait]
pub trait AuthenticationProvider
where
    Self: Send + Sync,
{
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError>;

    fn supports(&self, authentication: &str) -> bool;
}
