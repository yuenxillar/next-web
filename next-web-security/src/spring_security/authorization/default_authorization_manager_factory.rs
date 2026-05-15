use std::sync::Arc;

use crate::{
    access::hierarchicalroles::{
        null_role_hierarchy::NullRoleHierarchy, role_hierarchy::RoleHierarchy,
    },
    authorization::{
        all_authorities_authorization_manager::AllAuthoritiesAuthorizationManager,
        authenticated_authorization_manager::AuthenticatedAuthorizationManager,
        authentication_trust_resolver::{
            AuthenticationTrustResolver, DefaultAuthenticationTrustResolver,
        },
        authority_authorization_manager::AuthorityAuthorizationManager,
        authorization_manager::AuthorizationManager,
        authorization_manager_factory::AuthorizationManagerFactory,
        authorization_managers::AuthorizationManagers,
        authorization_decision::AuthorizationDecision,
    },
};

const DEFAULT_ROLE_PREFIX: &str = "ROLE_";

pub struct DefaultAuthorizationManagerFactory<T: Clone + Send + Sync + 'static> {
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
    role_hierarchy: Arc<dyn RoleHierarchy>,
    role_prefix: String,
    additional_authorization: Option<Arc<dyn AuthorizationManager<T>>>,
}

impl<T: Clone + Send + Sync + 'static> DefaultAuthorizationManagerFactory<T> {
    pub fn new() -> Self {
        Self {
            trust_resolver: Arc::new(DefaultAuthenticationTrustResolver::default()),
            role_hierarchy: Arc::new(NullRoleHierarchy),
            role_prefix: DEFAULT_ROLE_PREFIX.to_string(),
            additional_authorization: None,
        }
    }

    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    pub fn set_role_prefix(&mut self, role_prefix: impl Into<String>) {
        self.role_prefix = role_prefix.into();
    }

    pub fn set_additional_authorization(
        &mut self,
        additional_authorization: Option<Arc<dyn AuthorizationManager<T>>>,
    ) {
        self.additional_authorization = additional_authorization;
    }

    fn with_additional(
        &self,
        manager: Arc<dyn AuthorizationManager<T>>,
    ) -> Arc<dyn AuthorizationManager<T>> {
        match &self.additional_authorization {
            Some(additional) => Arc::new(AuthorizationManagers::all_of(vec![
                additional.clone(),
                manager,
            ])),
            None => manager,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Default for DefaultAuthorizationManagerFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + Sync + 'static> AuthorizationManagerFactory<T>
    for DefaultAuthorizationManagerFactory<T>
{
    fn has_any_role(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthorityAuthorizationManager::<T>::has_any_role(&self.role_prefix, roles.to_vec());
        let mut manager = manager;
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_all_roles(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let authorities: Vec<String> = roles.iter().map(|r| format!("{}{}", self.role_prefix, r)).collect();
        let mut manager = AllAuthoritiesAuthorizationManager::<T>::has_all_authorities(authorities);
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_authority(&self, authority: &str) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthorityAuthorizationManager::<T>::has_authority(authority);
        let mut manager = manager;
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_any_authority(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthorityAuthorizationManager::<T>::has_any_authority(authorities.to_vec());
        let mut manager = manager;
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_all_authorities(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AllAuthoritiesAuthorizationManager::<T>::has_all_authorities(authorities.to_vec());
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn authenticated(&self) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthenticatedAuthorizationManager::authenticated();
        self.with_additional(Arc::new(manager))
    }

    fn fully_authenticated(&self) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthenticatedAuthorizationManager::fully_authenticated();
        self.with_additional(Arc::new(manager))
    }

    fn remember_me(&self) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthenticatedAuthorizationManager::remember_me();
        self.with_additional(Arc::new(manager))
    }

    fn anonymous(&self) -> Arc<dyn AuthorizationManager<T>> {
        let manager = AuthenticatedAuthorizationManager::anonymous();
        self.with_additional(Arc::new(manager))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        authorization::{
            authorization_manager::AuthorizationManager,
            authorization_manager_factory::AuthorizationManagerFactory,
            default_authorization_manager_factory::DefaultAuthorizationManagerFactory,
        },
        core::{
            authority_utils::AuthorityUtils, simple_authentication::SimpleAuthentication,
        },
    };

    #[tokio::test]
    async fn test_factory_has_role() {
        let factory = DefaultAuthorizationManagerFactory::<()>::new();
        let manager = factory.has_role("USER");

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
    async fn test_factory_permit_all() {
        let factory = DefaultAuthorizationManagerFactory::<()>::new();
        let manager = factory.permit_all();

        let auth = Box::new(
            SimpleAuthentication::builder()
                .principal("alice")
                .authenticated(false)
                .build(),
        );

        let result = manager.check(auth, ()).await;
        assert!(result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn test_factory_deny_all() {
        let factory = DefaultAuthorizationManagerFactory::<()>::new();
        let manager = factory.deny_all();

        let auth = Box::new(
            SimpleAuthentication::builder()
                .principal("alice")
                .authorities(AuthorityUtils::create_authority_list(["ROLE_ADMIN"]))
                .authenticated(true)
                .build(),
        );

        let result = manager.check(auth, ()).await;
        assert!(!result.unwrap().is_granted());
    }
}
