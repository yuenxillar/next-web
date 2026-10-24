use std::sync::Arc;

use crate::authorization::{
    authority_authorization_manager::ROLE_PREFIX, AllAuthoritiesAuthorizationManager,
    AuthenticatedAuthorizationManager, AuthorityAuthorizationManager, AuthorizationManager,
    SingleResultAuthorizationManager,
};

/// A factory for creating different kinds of AuthorizationManager instances.
pub trait AuthorizationManagerFactory<T>
where
    Self: Send + Sync,
    T: Clone + Send + Sync + 'static,
{
    /// Create an AuthorizationManager that allows anyone.
    fn permit_all(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(SingleResultAuthorizationManager::<T>::permit_all())
    }

    /// Creates an AuthorizationManager that does not allow anyone.
    fn deny_all(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(SingleResultAuthorizationManager::<T>::deny_all())
    }

    /// Creates an AuthorizationManager that requires users to have the specified role.
    fn has_role(&self, role: &str) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthorityAuthorizationManager::has_role(role))
    }

    /// Creates an AuthorizationManager that requires users to have one of many roles.
    fn has_any_role(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthorityAuthorizationManager::has_any_role(
            ROLE_PREFIX,
            roles.to_vec(),
        ))
    }

    /// Creates an AuthorizationManager that requires users to have all the provided roles.
    fn has_all_roles(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AllAuthoritiesAuthorizationManager::has_all_roles(roles))
    }

    /// Creates an AuthorizationManager that requires users to have the specified authority.
    fn has_authority(&self, authority: &str) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthorityAuthorizationManager::has_authority(authority))
    }

    /// Creates an AuthorizationManager that requires users to have one of many authorities.
    fn has_any_authority(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthorityAuthorizationManager::has_any_authority(
            authorities.to_vec(),
        ))
    }

    /// Creates an AuthorizationManager that requires users to have all the provided authorities.
    fn has_all_authorities(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AllAuthoritiesAuthorizationManager::has_all_authorities(
            authorities,
        ))
    }

    /// Creates an AuthorizationManager that allows any authenticated user.
    fn authenticated(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthenticatedAuthorizationManager::authenticated())
    }

    /// Creates an AuthorizationManager that allows users who have authenticated and were not
    fn fully_authenticated(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthenticatedAuthorizationManager::fully_authenticated())
    }

    /// Creates an AuthorizationManager that allows users that have been remembered.
    fn remember_me(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthenticatedAuthorizationManager::remember_me())
    }

    /// Creates an AuthorizationManager that allows only anonymous users.
    fn anonymous(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(AuthenticatedAuthorizationManager::anonymous())
    }
}
