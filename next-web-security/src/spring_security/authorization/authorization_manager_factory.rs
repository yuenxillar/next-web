use std::sync::Arc;

use crate::authorization::{
    all_authorities_authorization_manager::AllAuthoritiesAuthorizationManager,
    authenticated_authorization_manager::AuthenticatedAuthorizationManager,
    authority_authorization_manager::AuthorityAuthorizationManager,
    authorization_manager::AuthorizationManager,
    single_result_authorization_manager::SingleResultAuthorizationManager,
};

/// A factory for creating different kinds of AuthorizationManager instances.
pub trait AuthorizationManagerFactory<T>: Send + Sync
where
    T: Clone + Send + Sync + 'static,
{
    fn permit_all(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(SingleResultAuthorizationManager::<T>::permit_all())
    }

    fn deny_all(&self) -> Arc<dyn AuthorizationManager<T>> {
        Arc::new(SingleResultAuthorizationManager::<T>::deny_all())
    }

    fn has_role(&self, role: &str) -> Arc<dyn AuthorizationManager<T>> {
        self.has_any_role(&[role.to_string()])
    }

    fn has_any_role(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>>;

    fn has_all_roles(&self, roles: &[String]) -> Arc<dyn AuthorizationManager<T>>;

    fn has_authority(&self, authority: &str) -> Arc<dyn AuthorizationManager<T>>;

    fn has_any_authority(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>>;

    fn has_all_authorities(&self, authorities: &[String]) -> Arc<dyn AuthorizationManager<T>>;

    fn authenticated(&self) -> Arc<dyn AuthorizationManager<T>>;

    fn fully_authenticated(&self) -> Arc<dyn AuthorizationManager<T>>;

    fn remember_me(&self) -> Arc<dyn AuthorizationManager<T>>;

    fn anonymous(&self) -> Arc<dyn AuthorizationManager<T>>;
}
