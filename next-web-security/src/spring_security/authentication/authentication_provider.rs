use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::core::{authentication_error::AuthenticationError, Authentication};

#[async_trait]
pub trait AuthenticationProvider
where
    Self: Send + Sync,
{
    /// Performs authentication with the same contract as AuthenticationManager.authenticate(Authentication) .
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError>;

    fn supports(&self, authentication: TypeId) -> bool;
}
