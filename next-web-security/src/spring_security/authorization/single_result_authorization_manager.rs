use std::marker::PhantomData;

use next_web_core::async_trait;

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        AuthorizationResult,
    },
    core::Authentication,
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
impl<T: Send + Sync + 'static> AuthorizationManager<T> for SingleResultAuthorizationManager<T> {
    // async fn check(
    //     &self,
    //     _authentication: Box<dyn Authentication>,
    //     _object: C,
    // ) -> Option<AuthorizationDecision> {
    //     Some(self.result.clone())
    // }

    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
    }
}
