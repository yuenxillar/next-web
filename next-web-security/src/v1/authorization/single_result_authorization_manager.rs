use std::{
    any::Any,
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

use next_web_core::{async_trait, error::BoxError};

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        AuthorizationResult,
    },
    core::Authentication,
};

static DENY: LazyLock<Arc<AuthorizationDecision>> =
    LazyLock::new(|| Arc::new(AuthorizationDecision::new(false)));
static PERMIT: LazyLock<Arc<AuthorizationDecision>> =
    LazyLock::new(|| Arc::new(AuthorizationDecision::new(true)));

/// An AuthorizationManager which creates permit-all and deny-all AuthorizationManager instances.
#[derive(Clone)]
pub struct SingleResultAuthorizationManager<C> {
    result: Arc<dyn AuthorizationResult>,
    _marker: PhantomData<C>,
}

impl<C> SingleResultAuthorizationManager<C> {
    pub fn new(result: Arc<dyn AuthorizationResult>) -> Self {
        Self {
            result,
            _marker: PhantomData,
        }
    }

    pub fn deny_all() -> Self {
        Self::new(DENY.clone())
    }

    pub fn permit_all() -> Self {
        Self::new(PERMIT.clone())
    }
}

#[async_trait]
impl<C> AuthorizationManager<C> for SingleResultAuthorizationManager<C>
where
    C: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        _authentication: &dyn Authentication,
        _var: &C,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        if (self.result.as_ref() as &dyn Any)
            .downcast_ref::<AuthorizationDecision>()
            .is_none()
        {
            return Err("result should be AuthorizationDecision".into());
        }

        Ok(Some(self.result.clone()))
    }
}
