use std::{marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        AuthorizationResult,
    },
    core::Authentication,
};

/// A factory class to create an AuthorizationManager instances.
pub struct AuthorizationManagers;

impl AuthorizationManagers {
    /// Creates an AuthorizationManager that grants access if at least one AuthorizationManager
    /// granted or abstained, if `managers` are empty then denied decision is returned.
    pub fn any_of<T>(
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AnyOfAuthorizationManager<T> {
        AnyOfAuthorizationManager::new(AuthorizationDecision::new(false), managers)
    }

    /// Creates an AuthorizationManager that grants access if all AuthorizationManager
    /// granted or abstained, if `managers` are empty then granted decision is returned.
    pub fn all_of<T>(
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AllOfAuthorizationManager<T> {
        AllOfAuthorizationManager::new(AuthorizationDecision::new(true), managers)
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
    all_abstain_default_decision: AuthorizationDecision,
    managers: Vec<Arc<dyn AuthorizationManager<T>>>,
}

impl<T> AnyOfAuthorizationManager<T> {
    pub fn new(
        all_abstain_default_decision: AuthorizationDecision,
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> Self {
        Self {
            all_abstain_default_decision,
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
    ) -> Option<Box<dyn AuthorizationResult>> {
        let mut results: Vec<Box<dyn AuthorizationResult>> = Vec::new();
        for manager in &self.managers {
            let result = manager.authorize(authentication, var).await;
            let Some(result) = result else {
                continue;
            };
            if result.is_granted() {
                return Some(result);
            }
            results.push(result);
        }

        if results.is_empty() {
            return Some(Box::new(self.all_abstain_default_decision.clone()));
        }

        Some(Box::new(CompositeAuthorizationDecision::new(
            false, results,
        )))
    }
}

pub struct AllOfAuthorizationManager<T> {
    all_abstain_default_decision: AuthorizationDecision,
    managers: Vec<Arc<dyn AuthorizationManager<T>>>,
}

impl<T> AllOfAuthorizationManager<T> {
    pub fn new(
        all_abstain_default_decision: AuthorizationDecision,
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> Self {
        Self {
            all_abstain_default_decision,
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
    ) -> Option<Box<dyn AuthorizationResult>> {
        let mut results: Vec<Box<dyn AuthorizationResult>> = Vec::new();
        for manager in &self.managers {
            let result = manager.authorize(authentication, var).await;
            let Some(result) = result else {
                continue;
            };
            if !result.is_granted() {
                return Some(result);
            }
            results.push(result);
        }

        if results.is_empty() {
            return Some(Box::new(self.all_abstain_default_decision.clone()));
        }

        Some(Box::new(CompositeAuthorizationDecision::new(true, results)))
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
    ) -> Option<Box<dyn AuthorizationResult>> {
        self.manager
            .authorize(authentication, var)
            .await
            .map(|result| {
                Box::new(NotAuthorizationDecision::new(result)) as Box<dyn AuthorizationResult>
            })
    }
}

/// An AuthorizationDecision that carries the AuthorizationResults which contributed to the decision.
#[derive(Clone)]
struct CompositeAuthorizationDecision {
    results: Vec<Box<dyn AuthorizationResult>>,
    inner: AuthorizationDecision,
}

impl CompositeAuthorizationDecision {
    fn new(granted: bool, results: Vec<Box<dyn AuthorizationResult>>) -> Self {
        Self {
            inner: AuthorizationDecision::new(granted),
            results,
        }
    }
}

impl AuthorizationResult for CompositeAuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.inner.is_granted()
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
    result: Box<dyn AuthorizationResult>,
    inner: AuthorizationDecision,
}

impl NotAuthorizationDecision {
    fn new(result: Box<dyn AuthorizationResult>) -> Self {
        Self {
            inner: AuthorizationDecision::new(!result.is_granted()),
            result,
        }
    }
}

impl AuthorizationResult for NotAuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.inner.is_granted()
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::async_trait;

    use crate::core::SimpleAuthentication;

    use super::*;

    #[derive(Clone)]
    struct GrantManager;

    #[derive(Clone)]
    struct DenyManager;

    #[derive(Clone)]
    struct AbstainManager;

    #[async_trait]
    impl AuthorizationManager<()> for GrantManager {
        async fn authorize(
            &self,
            _authentication: &dyn Authentication,
            _var: &(),
        ) -> Option<Box<dyn AuthorizationResult>> {
            Some(Box::new(AuthorizationDecision::new(true)))
        }
    }

    #[async_trait]
    impl AuthorizationManager<()> for DenyManager {
        async fn authorize(
            &self,
            _authentication: &dyn Authentication,
            _var: &(),
        ) -> Option<Box<dyn AuthorizationResult>> {
            Some(Box::new(AuthorizationDecision::new(false)))
        }
    }

    #[async_trait]
    impl AuthorizationManager<()> for AbstainManager {
        async fn authorize(
            &self,
            _authentication: &dyn Authentication,
            _var: &(),
        ) -> Option<Box<dyn AuthorizationResult>> {
            None
        }
    }

    fn authentication() -> Arc<dyn Authentication> {
        let mut builder = SimpleAuthentication::default().to_builder();
        builder.principal(Some(Arc::new(String::from("alice"))));
        builder.authenticated(true);
        builder.build()
    }

    #[tokio::test]
    async fn any_of_grants_if_any_manager_grants() {
        let manager =
            AuthorizationManagers::any_of(vec![Arc::new(DenyManager), Arc::new(GrantManager)]);
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn any_of_denies_when_all_managers_deny() {
        let manager =
            AuthorizationManagers::any_of(vec![Arc::new(DenyManager), Arc::new(DenyManager)]);
        let mut var = ();

        let result = manager
            .authorize(authentication().as_ref(), &mut var)
            .await
            .unwrap();

        assert!(!result.is_granted());
        assert!((result.as_ref() as &dyn std::any::Any)
            .downcast_ref::<CompositeAuthorizationDecision>()
            .is_some());
    }

    #[tokio::test]
    async fn any_of_uses_default_decision_when_all_managers_abstain() {
        let manager =
            AuthorizationManagers::any_of(vec![Arc::new(AbstainManager), Arc::new(AbstainManager)]);
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(!result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn any_of_ignores_abstaining_managers() {
        let manager =
            AuthorizationManagers::any_of(vec![Arc::new(AbstainManager), Arc::new(GrantManager)]);
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn all_of_grants_if_all_managers_grant() {
        let manager =
            AuthorizationManagers::all_of(vec![Arc::new(GrantManager), Arc::new(GrantManager)]);
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn all_of_denies_if_any_manager_denies() {
        let manager =
            AuthorizationManagers::all_of(vec![Arc::new(GrantManager), Arc::new(DenyManager)]);
        let mut var = ();

        let result = manager
            .authorize(authentication().as_ref(), &mut var)
            .await
            .unwrap();

        assert!(!result.is_granted());
        assert!((result.as_ref() as &dyn std::any::Any)
            .downcast_ref::<CompositeAuthorizationDecision>()
            .is_some());
    }

    #[tokio::test]
    async fn all_of_uses_default_decision_when_all_managers_abstain() {
        let manager =
            AuthorizationManagers::all_of(vec![Arc::new(AbstainManager), Arc::new(AbstainManager)]);
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn all_of_grants_when_managers_are_empty() {
        let manager = AuthorizationManagers::all_of(vec![]);
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn not_reverses_the_decision() {
        let manager = AuthorizationManagers::not(Arc::new(DenyManager));
        let mut var = ();

        let result = manager
            .authorize(authentication().as_ref(), &mut var)
            .await
            .unwrap();

        assert!(result.is_granted());
        assert!((result.as_ref() as &dyn std::any::Any)
            .downcast_ref::<NotAuthorizationDecision>()
            .is_some());
    }

    #[tokio::test]
    async fn not_abstains_when_manager_abstains() {
        let manager = AuthorizationManagers::not(Arc::new(AbstainManager));
        let mut var = ();

        let result = manager.authorize(authentication().as_ref(), &mut var).await;

        assert!(result.is_none());
    }
}
