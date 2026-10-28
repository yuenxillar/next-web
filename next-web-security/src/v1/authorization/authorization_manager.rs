use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::{
    access::AccessDeniedError,
    authorization::{AuthorizationDeniedError, AuthorizationResult},
    core::Authentication,
};

/// An Authorization manager which can determine if an Authentication has access to a specific object.
#[async_trait]
pub trait AuthorizationManager<T>
where
    Self: Send + Sync,
    T: Send + Sync,
{
    /// Determines if access is granted for a specific authentication and object.
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError>;

    /// Determines if access should be granted for a specific authentication and object.
    async fn verify(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<(), AccessDeniedError> {
        if let Some(result) = self.authorize(authentication, var).await.ok().flatten() {
            if !result.is_granted() {
                return Err(AccessDeniedError::AuthorizationDenied(
                    AuthorizationDeniedError::new("Access Denied", result),
                ));
            }
        }

        Ok(())
    }
}
