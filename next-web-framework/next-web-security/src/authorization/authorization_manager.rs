use std::future::Future;

use next_web_core::async_trait;

use crate::{
    access::{
        access_denied_error::AccessDeniedError,
        intercept::request_authorization_context::RequestAuthorizationContext,
    },
    authorization::authorization_decision::AuthorizationDecision,
    core::authentication::Authentication,
};

#[async_trait]
pub trait AuthorizationManager<T>
where
    Self: Send,
    T: Send + 'static
{
    async fn check(&self, authentication: Box<dyn Authentication>, object: T) -> Option<AuthorizationDecision>;
    async fn verify(&self, authentication: Box<dyn Authentication>, object: T) -> Result<(), AccessDeniedError>
    {
        let decision = self.check(authentication, object).await;
        if let Some(decision) = decision {
            if decision.is_granted() {
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
    async fn check(
        &self,
        authentication: Box<dyn Authentication>,
        object: RequestAuthorizationContext,
    ) -> Option<AuthorizationDecision>
    {
        Some(AuthorizationDecision::new(self.0))
    }
}
