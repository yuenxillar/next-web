use next_web_core::async_trait;

use crate::{
    access::{
        access_denied_error::AccessDeniedError,
        intercept::request_authorization_context::RequestAuthorizationContext,
    },
    authorization::AuthorizationResult,
    core::Authentication,
};

#[async_trait]
pub trait AuthorizationManager<T>
where
    Self: Send + Sync,
    T: Send + Sync,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>>;

    async fn verify(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Result<(), AccessDeniedError> {
        let decision = self.authorize(authentication, var).await;
        if let Some(decision) = decision {
            if !decision.is_granted() {
                return Err(AccessDeniedError::from("Access Denied"));
            }
        }

        Ok(())
    }
}

pub struct DefaultAuthorizationManager(pub bool);

#[async_trait]
impl AuthorizationManager<RequestAuthorizationContext> for DefaultAuthorizationManager {
    #[allow(unused_variables)]
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut RequestAuthorizationContext,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
    }
}
