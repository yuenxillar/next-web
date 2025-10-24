use std::{fmt::Display, sync::Arc};

use next_web_core::{clone_box, traits::required::Required};

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
        authentication_token::AuthenticationToken, authenticator::Authenticator,
        pam::modular_realm_authenticator::ModularRealmAuthenticator,
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
    mgt::{
        default_subject_dao::DefaultSubjectDAO, default_subject_factory::DefaultSubjectFactory,
        remember_me_manager::RememberMeManager, security_manager::SecurityManager,
        sessions_security_manager::SessionsSecurityManager, subject_dao::SubjectDAO,
        subject_factory::SubjectFactory,
    },
    realm::{Realm, simple_account_realm::SimpleAccountRealm},
    session::{
        Session, SessionError, SessionId,
        mgt::{
            default_session_manager::DefaultSessionManager, session_context::SessionContext,
            session_manager::SessionManager,
        },
    },
    subject::{
        Subject, principal_collection::PrincipalCollection, subject_context::SubjectContext,
        support::default_subject_context::DefaultSubjectContext,
    },
};

#[derive(Clone)]
pub struct DefaultSecurityManager<
    M = (),
    D = DefaultSubjectDAO,
    F = DefaultSubjectFactory,
    S = DefaultSessionManager,
    A = ModularRealmAuthorizer,
    T = ModularRealmAuthenticator,
    R = SimpleAccountRealm,
    C = DefaultCacheManager,
    B = DefaultEventBus,
> {
    remember_me_manager: Option<M>,
    subject_dao: D,
    subject_factory: F,

    sessions_security_manager: SessionsSecurityManager<S, A, T, R, C, B>,
}

impl From<SimpleAccountRealm> for DefaultSecurityManager {
    fn from(single_realm: SimpleAccountRealm) -> Self {
        let mut manager = Self {
            remember_me_manager: Default::default(),
            subject_dao: Default::default(),
            subject_factory: Default::default(),
            sessions_security_manager: Default::default(),
        };
        manager
            .sessions_security_manager
            .get_mut_object()
            .get_mut_object()
            .get_mut_object()
            .set_realm(single_realm);

        manager
    }
}

impl From<Vec<SimpleAccountRealm>> for DefaultSecurityManager {
    fn from(realms: Vec<SimpleAccountRealm>) -> Self {
        let mut manager = Self {
            remember_me_manager: Default::default(),
            subject_dao: Default::default(),
            subject_factory: Default::default(),
            sessions_security_manager: Default::default(),
        };

        manager
            .sessions_security_manager
            .get_mut_object()
            .get_mut_object()
            .get_mut_object()
            .set_realms(realms);

        manager
    }
}

impl<M, D, F, S, A, T, R, C, B> DefaultSecurityManager<M, D, F, S, A, T, R, C, B> {
    pub fn get_subject_factory(&self) -> &F {
        &self.subject_factory
    }

    pub fn set_subject_factory(&mut self, subject_factory: F) {
        self.subject_factory = subject_factory;
    }

    pub fn get_subject_dao(&self) -> &D {
        &self.subject_dao
    }

    pub fn set_subject_dao(&mut self, subject_dao: D) {
        self.subject_dao = subject_dao;
    }

    pub fn get_remember_me_manager(&self) -> Option<&M> {
        self.remember_me_manager.as_ref()
    }

    pub fn set_remember_me_manager(&mut self, remember_me_manager: M) {
        self.remember_me_manager = Some(remember_me_manager);
    }

    pub fn create_subject_context(&self) -> DefaultSubjectContext {
        DefaultSubjectContext::default()
    }

    pub fn on_successful_login(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
        subject: &dyn Subject,
    ) {
    }

    pub fn on_failed_login(
        &self,
        token: &dyn AuthenticationToken,
        error: &AuthenticationError,
        subject: &dyn Subject,
    ) {
    }
}

impl<M, D, F, S, A, T, R, C, B> DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: RememberMeManager,
    D: SubjectDAO,
    F: SubjectFactory,
    S: SessionManager,
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + EventBusAware<B>,
    C: Clone,
    B: EventBus + Clone,
{
    fn _create_subject(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) -> Box<dyn Subject> {
        let mut context = self.create_subject_context();
        context.set_authenticated(true);
        context.set_authentication_token(clone_box(token));
        context.set_authentication_info(clone_box(info));

        self.create_subject(context)
    }
}

impl<M, D, F, S, A, T, R, C, B> Authorizer for DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: Send + Sync,
    D: Send + Sync,
    F: Send + Sync,
    S: Send + Sync,
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    fn is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        self.sessions_security_manager
            .get_object()
            .is_permitted(principal, permission)
    }

    fn is_permitted_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        self.sessions_security_manager
            .get_object()
            .is_permitted_from_str(principal, permission)
    }

    fn is_permitted_from_str_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        self.sessions_security_manager
            .get_object()
            .is_permitted_from_str_list(principal, permissions)
    }

    fn is_permitted_from_permission_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        self.sessions_security_manager
            .get_object()
            .is_permitted_from_permission_list(principal, permissions)
    }

    fn is_permitted_all(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        self.sessions_security_manager
            .get_object()
            .is_permitted_all(principal, permissions)
    }

    fn is_permitted_all_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        self.sessions_security_manager
            .get_object()
            .is_permitted_all_from_str(principal, permissions)
    }

    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        self.sessions_security_manager
            .get_object()
            .check_permission(principal, permission)
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        self.sessions_security_manager
            .get_object()
            .check_permission_from_str(principal, permission)
    }

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        self.sessions_security_manager
            .get_object()
            .check_permissions(principals, permissions)
    }

    fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.sessions_security_manager
            .get_object()
            .check_permissions_from_str(principals, permissions)
    }

    fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool {
        self.sessions_security_manager
            .get_object()
            .has_role(principal, role_identifier)
    }

    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        self.sessions_security_manager
            .get_object()
            .has_roles(principal, role_identifiers)
    }

    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        self.sessions_security_manager
            .get_object()
            .has_all_roles(principal, role_identifiers)
    }

    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        self.sessions_security_manager
            .get_object()
            .check_role(principal, role)
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.sessions_security_manager
            .get_object()
            .check_roles(principal, roles)
    }
}

impl<M, D, F, S, A, T, R, C, B> Authenticator for DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: Send + Sync,
    D: Send + Sync,
    F: Send + Sync,
    S: Send + Sync,
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + Clone,
    B: EventBus + Clone,
{
    fn authenticate(
        &self,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<&dyn AuthenticationInfo, AuthenticationError> {
        self.sessions_security_manager
            .get_object()
            .get_object()
            .authenticate(authentication_token)
    }
}

impl<M, D, F, S, A, T, R, C, B> SessionManager for DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: Send + Sync,
    D: Send + Sync,
    F: Send + Sync,
    S: SessionManager,
    A: Send + Sync,
    T: Send + Sync,
    R: Send + Sync,
    C: Send + Sync,
    B: Send + Sync,
{
    fn start(&self, context: &dyn SessionContext) -> Result<Box<dyn Session>, AuthorizationError> {
        self.sessions_security_manager.start(context)
    }

    fn get_session(&self, id: SessionId) -> Result<Arc<dyn Session>, SessionError> {
        self.sessions_security_manager.get_session(id)
    }
}

impl<M, D, F, S, A, T, R, C, B> SecurityManager
    for DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: RememberMeManager,
    D: SubjectDAO,
    F: SubjectFactory,
    S: SessionManager,
    A: Authorizer,
    T: Authenticator,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    R: Clone,
    C: CacheManager + EventBusAware<B>,
    C: Clone,
    B: EventBus + Clone,
{
    fn login(
        &self,
        subject: &dyn Subject,
        token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn Subject>, AuthenticationError> {
        let info = match self.authenticate(token) {
            Ok(info) => info,
            Err(error) => {
                if let Err(err) = self.on_failed_login(token, error, subject) {}
                return Err(error);
            }
        };

        let logged_in = self._create_subject(token, info);

        self.on_successful_login(token, info, logged_in.as_ref());

        Ok(logged_in)
    }

    fn logout(&self, subject: &dyn Subject) -> Result<(), next_web_core::error::BoxError> {
        todo!()
    }

    fn create_subject<CTX: SubjectContext>(&self, context: CTX) -> Box<dyn Subject> {
        todo!()
    }
}

impl Required<SessionsSecurityManager> for DefaultSecurityManager {
    fn get_object(&self) -> &SessionsSecurityManager {
        &self.sessions_security_manager
    }

    fn get_mut_object(&mut self) -> &mut SessionsSecurityManager {
        &mut self.sessions_security_manager
    }
}

impl<M, D, F, S, A, T, R, C, B> Default for DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: Default,
    D: Default,
    F: Default,
    S: SessionManager + CacheManagerAware<C> + EventBusAware<B>,
    S: Default + Clone,
    A: Default + Authorizer,
    T: Default + Authenticator,
    R: Default + Clone,
    R: Realm + CacheManagerAware<C> + EventBusAware<B>,
    C: Default + Clone,
    C: CacheManager + EventBusAware<B>,
    B: Default + Clone + EventBus,
{
    fn default() -> Self {
        let sessions_security_manager = SessionsSecurityManager::default();
        Self {
            remember_me_manager: Default::default(),
            subject_dao: Default::default(),
            subject_factory: Default::default(),

            sessions_security_manager,
        }
    }
}

impl<M, D, F, S, A, T, R, C, B> Display for DefaultSecurityManager<M, D, F, S, A, T, R, C, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultSecurityManager -> []")
    }
}
