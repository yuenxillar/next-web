use next_web_core::async_trait;

use crate::{
    access::AccessDeniedError, authorization::authorization_result::AuthorizationResult,
    core::Authentication,
};

/// Reactive equivalent of `AuthorizationManager`.
#[async_trait]
pub trait ReactiveAuthorizationManager<T>
where
    Self: Send + Sync,
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: Box<dyn Authentication>,
        object: T,
    ) -> Option<Box<dyn AuthorizationResult>>;

    async fn verify(
        &self,
        authentication: Box<dyn Authentication>,
        object: T,
    ) -> Result<(), AccessDeniedError> {
        let result = self.authorize(authentication, object).await;
        match result {
            Some(r) if r.is_granted() => Ok(()),
            _ => Err(AccessDeniedError::from("Access Denied")),
        }
    }
}
