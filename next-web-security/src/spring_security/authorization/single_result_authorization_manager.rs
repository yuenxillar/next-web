use std::marker::PhantomData;

use next_web_core::async_trait;

use crate::{
    authorization::{authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager},
    core::authentication::Authentication,
};

/// An AuthorizationManager that always returns the same single result.
pub struct SingleResultAuthorizationManager<C> {
    result: AuthorizationDecision,
    _marker: PhantomData<C>,
}

impl<C> SingleResultAuthorizationManager<C> {
    pub fn new(granted: bool) -> Self {
        Self {
            result: AuthorizationDecision::new(granted),
            _marker: PhantomData,
        }
    }

    pub fn permit_all() -> Self {
        Self::new(true)
    }

    pub fn deny_all() -> Self {
        Self::new(false)
    }
}

#[async_trait]
impl<C: Send + Sync + 'static> AuthorizationManager<C> for SingleResultAuthorizationManager<C> {
    async fn check(
        &self,
        _authentication: Box<dyn Authentication>,
        _object: C,
    ) -> Option<AuthorizationDecision> {
        Some(self.result.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        authorization::{
            authorization_manager::AuthorizationManager,
            single_result_authorization_manager::SingleResultAuthorizationManager,
        },
        core::{authority_utils::AuthorityUtils, simple_authentication::SimpleAuthentication},
    };

    #[tokio::test]
    async fn test_permit_all() {
        let manager: SingleResultAuthorizationManager<()> =
            SingleResultAuthorizationManager::permit_all();
        let auth = Box::new(
            SimpleAuthentication::builder()
                .principal("alice")
                .authorities(AuthorityUtils::create_authority_list(["ROLE_USER"]))
                .authenticated(true)
                .build(),
        );

        let result = manager.check(auth, ()).await;
        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn test_deny_all() {
        let manager: SingleResultAuthorizationManager<()> =
            SingleResultAuthorizationManager::deny_all();
        let auth = Box::new(
            SimpleAuthentication::builder()
                .principal("alice")
                .authorities(AuthorityUtils::create_authority_list(["ROLE_USER"]))
                .authenticated(true)
                .build(),
        );

        let result = manager.check(auth, ()).await;
        assert!(!result.unwrap().is_granted());
    }
}
