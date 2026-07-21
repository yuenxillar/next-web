use std::{marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        AuthorizationResult,
    },
    core::Authentication,
};

pub struct AuthorizationManagers;

impl AuthorizationManagers {
    pub fn any_of<T>(
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AnyOfAuthorizationManager<T> {
        AnyOfAuthorizationManager::new(AuthorizationDecision::new(false), managers)
    }

    pub fn all_of<T>(
        managers: Vec<Arc<dyn AuthorizationManager<T>>>,
    ) -> AllOfAuthorizationManager<T> {
        AllOfAuthorizationManager::new(AuthorizationDecision::new(true), managers)
    }

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
    T: Clone + Send + Sync + 'static,
{
    // async fn check(
    //     &self,
    //     authentication: Box<dyn Authentication>,
    //     object: T,
    // ) -> Option<AuthorizationDecision> {
    //     let snapshot = SimpleAuthentication::builder_from(authentication.as_ref()).build();
    //     let mut denied = false;
    //     for manager in &self.managers {
    //         let candidate = SimpleAuthentication::builder_from(&snapshot).build();
    //         if let Some(decision) = manager.check(Box::new(candidate), object.clone()).await {
    //             if decision.is_granted() {
    //                 return Some(decision);
    //             }
    //             denied = true;
    //         }
    //     }
    //     Some(if denied {
    //         AuthorizationDecision::new(false)
    //     } else {
    //         self.all_abstain_default_decision.clone()
    //     })
    // }

    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
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
    T: Clone + Send + Sync + 'static,
{
    // async fn check(
    //     &self,
    //     authentication: Box<dyn Authentication>,
    //     object: T,
    // ) -> Option<AuthorizationDecision> {
    //     let snapshot = SimpleAuthentication::builder_from(authentication.as_ref()).build();
    //     let mut granted = false;
    //     for manager in &self.managers {
    //         let candidate = SimpleAuthentication::builder_from(&snapshot).build();
    //         if let Some(decision) = manager.check(Box::new(candidate), object.clone()).await {
    //             if !decision.is_granted() {
    //                 return Some(decision);
    //             }
    //             granted = true;
    //         }
    //     }
    //     Some(if granted {
    //         AuthorizationDecision::new(true)
    //     } else {
    //         self.all_abstain_default_decision.clone()
    //     })
    // }

    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
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
    // async fn check(
    //     &self,
    //     authentication: Box<dyn Authentication>,
    //     object: T,
    // ) -> Option<AuthorizationDecision> {
    //     self.manager
    //         .check(authentication, object)
    //         .await
    //         .map(|decision| AuthorizationDecision::new(!decision.is_granted()))
    // }

    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
    }
}
