use std::any::Any;

use next_web_core::traits::required::Required;

use crate::core::{
    authc::{
        authenticator::Authenticator, pam::modular_realm_authenticator::ModularRealmAuthenticator,
    },
    authz::{
        authorization_error::AuthorizationError, authorizer::Authorizer,
        modular_realm_authorizer::ModularRealmAuthorizer, permission::Permission,
    },
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    event::{
        event_bus::EventBus, event_bus_aware::EventBusAware,
        support::default_event_bus::DefaultEventBus,
    },
    mgt::authenticating_security_manager::AuthenticatingSecurityManager,
    realm::{Realm, simple_account_realm::SimpleAccountRealm},
    subject::principal_collection::PrincipalCollection,
    util::destroyable::Destroyable,
};

#[derive(Clone)]
pub struct AuthorizingSecurityManager<
    A = ModularRealmAuthorizer,
    T = ModularRealmAuthenticator,
    R = SimpleAccountRealm,
    C = DefaultCacheManager,
    B = DefaultEventBus,
> {
    authorizer: A,
    authenticating_security_manager: AuthenticatingSecurityManager<T, R, C, B>,
}

impl<A> AuthorizingSecurityManager<A>
where
    A: Authorizer,
{
    pub fn get_authorizer(&self) -> &A {
        &self.authorizer
    }

    pub fn set_authorizer(&mut self, authorizer: A) {
        self.authorizer = authorizer;
    }
}

impl<A, T, R, C, B> AuthorizingSecurityManager<A, T, R, C, B>
where
    A: Authorizer,
    A: Any,
    T: Authenticator + 'static,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    pub fn after_realms_set(&mut self) {
        self.authenticating_security_manager.after_realms_set();

        if let Some(authorizer) =
            (&mut self.authorizer as &mut dyn Any).downcast_mut::<ModularRealmAuthorizer>()
        {
            let realms = self
                .authenticating_security_manager
                .get_object()
                .get_realms()
                .clone();
            authorizer.set_realms(realms);
        }
    }
}
impl<A, T, R, C, B> Authorizer for AuthorizingSecurityManager<A, T, R, C, B>
where
    A: Authorizer,
    T: Send + Sync,
    R: Send + Sync,
    C: Send + Sync,
    B: Send + Sync,
{
    fn is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        self.authorizer.is_permitted(principal, permission)
    }

    fn is_permitted_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        self.authorizer.is_permitted_from_str(principal, permission)
    }

    fn is_permitted_from_str_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        self.authorizer
            .is_permitted_from_str_list(principal, permissions)
    }

    fn is_permitted_from_permission_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        self.authorizer
            .is_permitted_from_permission_list(principal, permissions)
    }

    fn is_permitted_all(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        self.authorizer.is_permitted_all(principal, permissions)
    }

    fn is_permitted_all_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        self.authorizer
            .is_permitted_all_from_str(principal, permissions)
    }

    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        self.authorizer.check_permission(principal, permission)
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        self.authorizer
            .check_permission_from_str(principal, permission)
    }

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        self.authorizer.check_permissions(principals, permissions)
    }

    fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.authorizer
            .check_permissions_from_str(principals, permissions)
    }

    fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool {
        self.authorizer.has_role(principal, role_identifier)
    }

    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        self.authorizer.has_roles(principal, role_identifiers)
    }

    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        self.authorizer.has_all_roles(principal, role_identifiers)
    }

    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        self.authorizer.check_role(principal, role)
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.authorizer.check_roles(principal, roles)
    }
}

impl<A, T, R, C, B> Destroyable for AuthorizingSecurityManager<A, T, R, C, B>
where
    A: Authorizer + Destroyable,
    T: Authenticator + Destroyable,
    R: Realm,
    C: CacheManager,
    B: Destroyable + Default + EventBus,
{
    fn destroy(self) {
        self.authorizer.destroy();
        self.authenticating_security_manager.destroy();
    }
}

impl<A, T, R, C, B> Required<AuthenticatingSecurityManager<T, R, C, B>>
    for AuthorizingSecurityManager<A, T, R, C, B>
where
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    fn get_object(&self) -> &AuthenticatingSecurityManager<T, R, C, B> {
        &self.authenticating_security_manager
    }

    fn get_mut_object(&mut self) -> &mut AuthenticatingSecurityManager<T, R, C, B> {
        &mut self.authenticating_security_manager
    }
}

impl<A, T, R, C, B> Default for AuthorizingSecurityManager<A, T, R, C, B>
where
    A: Authorizer + Default,
    T: Default,
    R: Default + Realm,
    C: Default + CacheManager + EventBusAware<B>,
    B: Default + Clone + EventBus,
{
    fn default() -> Self {
        Self {
            authorizer: Default::default(),
            authenticating_security_manager: Default::default(),
        }
    }
}
