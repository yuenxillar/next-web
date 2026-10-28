use std::{marker::PhantomData, ops::Deref, sync::Arc};

use next_web_core::{async_trait, error::BoxError};

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        AuthorizationResult,
    },
    core::Authentication,
};

/// A factory  to create an AuthorizationManager instances.
pub struct AuthorizationManagers;

impl AuthorizationManagers {
    /// Creates an AuthorizationManager that grants access if at least one AuthorizationManager
    /// granted or abstained, if `managers` are empty then denied decision is returned.
    pub fn any_of<T>(
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AnyOfAuthorizationManager<T> {
        Self::any_of_with_authorization_decision(AuthorizationDecision::new(false), managers)
    }

    /// Creates an AuthorizationManager that grants access if at least one AuthorizationManager granted,
    /// if managers are empty or abstained, a default AuthorizationDecision is returned.
    pub fn any_of_with_authorization_decision<T>(
        default_decision: AuthorizationDecision,
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AnyOfAuthorizationManager<T> {
        AnyOfAuthorizationManager::new(default_decision, managers)
    }

    /// Creates an AuthorizationManager that grants access if all AuthorizationManager
    /// granted or abstained, if `managers` are empty then granted decision is returned.
    pub fn all_of<T>(
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AllOfAuthorizationManager<T> {
        Self::all_of_with_authorization_decision(AuthorizationDecision::new(true), managers)
    }

    /// Creates an AuthorizationManager that grants access if all AuthorizationManagers granted, if
    /// managers are empty or abstained, a default AuthorizationDecision is returned.
    pub fn all_of_with_authorization_decision<T>(
        decision: AuthorizationDecision,
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AllOfAuthorizationManager<T> {
        AllOfAuthorizationManager::new(decision, managers)
    }

    /// Creates an AuthorizationManager that reverses whatever decision the given
    /// AuthorizationManager granted. If the given AuthorizationManager abstains, then the
    /// returned manager also abstains.
    pub fn not<T>(manager: Arc<dyn AuthorizationManager<T>>) -> NotAuthorizationManager<T> {
        NotAuthorizationManager {
            manager,
            _marker: PhantomData,
        }
    }
}

pub struct AnyOfAuthorizationManager<T> {
    all_abstain_default_decision: Arc<AuthorizationDecision>,
    managers: Vec<Arc<dyn AuthorizationManager<T>>>,
}

impl<T> AnyOfAuthorizationManager<T> {
    pub fn new(
        all_abstain_default_decision: AuthorizationDecision,
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> Self {
        Self {
            all_abstain_default_decision: Arc::new(all_abstain_default_decision),
            managers,
        }
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AnyOfAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let mut results = Vec::new();
        for manager in self.managers.iter() {
            let result = manager.authorize(authentication, var).await?;
            let Some(result) = result else {
                continue;
            };
            if result.is_granted() {
                return Ok(Some(result));
            }
            results.push(result);
        }

        if results.is_empty() {
            return Ok(Some(self.all_abstain_default_decision.clone()));
        }

        Ok(Some(Arc::new(CompositeAuthorizationDecision::new(
            false, results,
        ))))
    }
}

pub struct AllOfAuthorizationManager<T> {
    all_abstain_default_decision: Arc<AuthorizationDecision>,
    managers: Vec<Arc<dyn AuthorizationManager<T>>>,
}

impl<T> AllOfAuthorizationManager<T> {
    pub fn new(
        all_abstain_default_decision: AuthorizationDecision,
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> Self {
        Self {
            all_abstain_default_decision: Arc::new(all_abstain_default_decision),
            managers,
        }
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AllOfAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let mut results = Vec::new();
        for manager in self.managers.iter() {
            let result = manager.authorize(authentication, var).await?;
            let Some(result) = result else {
                continue;
            };
            if !result.is_granted() {
                return Ok(Some(result));
            }
            results.push(result);
        }

        if results.is_empty() {
            return Ok(Some(self.all_abstain_default_decision.clone()));
        }

        Ok(Some(Arc::new(CompositeAuthorizationDecision::new(
            true, results,
        ))))
    }
}

#[derive(Clone)]
pub struct NotAuthorizationManager<T> {
    manager: Arc<dyn AuthorizationManager<T>>,
    _marker: PhantomData<T>,
}

#[async_trait]
impl<T> AuthorizationManager<T> for NotAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let result = self.manager.authorize(authentication, var).await?;

        Ok(result.map(|result| {
            Arc::new(NotAuthorizationDecision::new(result)) as Arc<dyn AuthorizationResult>
        }))
    }
}

/// An AuthorizationDecision that carries the AuthorizationResults which contributed to the decision.
#[derive(Clone)]
struct CompositeAuthorizationDecision {
    results: Vec<Arc<dyn AuthorizationResult>>,
    base: AuthorizationDecision,
}

impl CompositeAuthorizationDecision {
    fn new(granted: bool, results: Vec<Arc<dyn AuthorizationResult>>) -> Self {
        Self {
            base: AuthorizationDecision::new(granted),
            results,
        }
    }
}

impl Deref for CompositeAuthorizationDecision {
    type Target = AuthorizationDecision;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::fmt::Debug for CompositeAuthorizationDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CompositeAuthorizationDecision [results={:?}]",
            self.results
                .iter()
                .map(|result| result.is_granted())
                .collect::<Vec<_>>()
        )
    }
}

/// An AuthorizationDecision that reverses the decision of the wrapped AuthorizationResult.
#[derive(Clone)]
struct NotAuthorizationDecision {
    result: Arc<dyn AuthorizationResult>,
    base: AuthorizationDecision,
}

impl NotAuthorizationDecision {
    fn new(result: Arc<dyn AuthorizationResult>) -> Self {
        Self {
            base: AuthorizationDecision::new(!result.is_granted()),
            result,
        }
    }
}

impl Deref for NotAuthorizationDecision {
    type Target = AuthorizationDecision;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::fmt::Debug for NotAuthorizationDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NotAuthorizationDecision [result={}]",
            self.result.is_granted()
        )
    }
}
