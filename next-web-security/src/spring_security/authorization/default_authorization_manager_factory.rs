use std::sync::Arc;

use crate::{
    access::hierarchicalroles::{NullRoleHierarchy, RoleHierarchy},
    authentication::AuthenticationTrustResolverImpl,
    authorization::{
        AllAuthoritiesAuthorizationManager, AuthenticatedAuthorizationManager,
        AuthenticationTrustResolver, AuthorityAuthorizationManager, AuthorizationManager,
        AuthorizationManagerFactory, AuthorizationManagers,
    },
};

/// A factory for creating different kinds of AuthorizationManager instances
pub struct DefaultAuthorizationManagerFactory<T>
where
    T: Clone + Send + Sync + 'static,
{
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
    role_hierarchy: Arc<dyn RoleHierarchy>,
    role_prefix: String,
    additional_authorization: Option<Arc<dyn AuthorizationManager<T>>>,
}

impl<T: Clone + Send + Sync + 'static> DefaultAuthorizationManagerFactory<T> {
    /// Sets the AuthenticationTrustResolver used to check the user's authentication.
    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }

    /// Sets the RoleHierarchy used to discover reachable authorities.
    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    /// Sets the prefix used to create an authority name from a role name. Can be an empty string.
    pub fn set_role_prefix(&mut self, role_prefix: impl Into<String>) {
        self.role_prefix = role_prefix.into();
    }

    /// Sets additional authorization to be applied to the returned AuthorizationManager for the following methods:
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
        Self {
            trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
            role_hierarchy: Arc::new(NullRoleHierarchy),
            role_prefix: "ROLE_".to_string(),
            additional_authorization: None,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> AuthorizationManagerFactory<T>
    for DefaultAuthorizationManagerFactory<T>
{
    fn has_any_role(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager =
            AuthorityAuthorizationManager::<T>::has_any_role(&self.role_prefix, roles.to_vec());
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_all_roles(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AllAuthoritiesAuthorizationManager::<T>::has_all_prefixed_authorities(
            &self.role_prefix,
            roles,
        );
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_authority(&self, authority: &str) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AuthorityAuthorizationManager::<T>::has_authority(authority);
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_any_authority(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager =
            AuthorityAuthorizationManager::<T>::has_any_authority(authorities.to_vec());
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn has_all_authorities(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager =
            AllAuthoritiesAuthorizationManager::<T>::has_all_authorities(authorities.to_vec());
        manager.set_role_hierarchy(self.role_hierarchy.clone());
        self.with_additional(Arc::new(manager))
    }

    fn authenticated(&self) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AuthenticatedAuthorizationManager::authenticated();
        manager.set_trust_resolver(self.trust_resolver.clone());
        self.with_additional(Arc::new(manager))
    }

    fn fully_authenticated(&self) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AuthenticatedAuthorizationManager::fully_authenticated();
        manager.set_trust_resolver(self.trust_resolver.clone());
        self.with_additional(Arc::new(manager))
    }

    fn remember_me(&self) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AuthenticatedAuthorizationManager::remember_me();
        manager.set_trust_resolver(self.trust_resolver.clone());
        self.with_additional(Arc::new(manager))
    }

    fn anonymous(&self) -> Arc<dyn AuthorizationManager<T>> {
        let mut manager = AuthenticatedAuthorizationManager::anonymous();
        manager.set_trust_resolver(self.trust_resolver.clone());
        self.with_additional(Arc::new(manager))
    }
}
