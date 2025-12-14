use next_web_core::async_trait;

use crate::core::{authentication::Authentication, authentication_error::AuthenticationError};

#[async_trait]
pub trait AuthenticationProvider
where
    Self: Send + Sync,
{
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<(), AuthenticationError>;

    fn supports(&self, authentication: &str) -> bool;
}
