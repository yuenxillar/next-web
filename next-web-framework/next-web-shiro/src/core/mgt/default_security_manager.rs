use std::{fmt::Display, sync::Arc};

use next_web_core::{clone_box, traits::required::Required};
use tracing::{debug, info, trace, warn};

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
        authentication_token::AuthenticationToken, authenticator::Authenticator,
        logout_aware::LogoutAware, pam::modular_realm_authenticator::ModularRealmAuthenticator,
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
            default_session_context::DefaultSessionContext,
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

impl<M, D, F, S, A, T, R, C, B> DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: RememberMeManager,
{
    pub fn get_subject_factory(&self) -> &F {
        &self.subject_factory
    }

    pub fn set_subject_factory(&mut self, subject_factory: F) {
        self.subject_factory = subject_factory;
    }

    pub fn get_subject_dao(&self) -> &D {
        &self.subject_dao
    }

    pub fn get_mut_subject_dao(&mut self) -> &mut D {
        &mut self.subject_dao
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

    pub fn remember_me_successful_login(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
        subject: &dyn Subject,
    ) {
        if let Some(rmm) = self.get_remember_me_manager() {
            rmm.on_successful_login(subject, token, info);
        }
    }

    pub fn remember_me_failed_login(
        &self,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
        subject: &dyn Subject,
    ) {
        if let Some(rmm) = self.get_remember_me_manager() {
            rmm.on_failed_login(subject, token, ae);
        }
    }

    pub fn remember_me_logout(&self, subject: &dyn Subject) {
        if let Some(rmm) = self.get_remember_me_manager() {
            rmm.on_logout(subject);
        }
    }

    pub fn on_successful_login(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
        subject: &dyn Subject,
    ) {
        self.remember_me_successful_login(token, info, subject);
    }

    pub fn on_failed_login(
        &self,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
        subject: &dyn Subject,
    ) -> Result<(), AuthenticationError> {
        self.remember_me_failed_login(token, ae, subject);
        Ok(())
    }

    pub fn before_logout(&self, subject: &dyn Subject) {
        self.remember_me_logout(subject);
    }

    pub fn ensure_security_manager(&self, _context: &dyn SubjectContext) {
        // TODO set security manager in context
    }

    pub fn get_session_id<'a>(&'a self, context: &'a dyn SubjectContext) -> &'a SessionId {
        context.get_session_id()
    }

    pub fn is_empty(&self, pc: &dyn PrincipalCollection) -> bool {
        pc.is_empty()
    }
}

impl<M, D, F, S, A, T, R, C, B> DefaultSecurityManager<M, D, F, S, A, T, R, C, B>
where
    M: RememberMeManager,
    D: SubjectDAO,
    F: SubjectFactory,
    S: SessionManager,
    A: Authorizer,
    T: Authenticator + LogoutAware,
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
        existing: Option<&dyn Subject>,
    ) -> Box<dyn Subject> {
        let mut context = self.create_subject_context();
        context.set_authenticated(true);
        context.set_authentication_token(clone_box(token));
        context.set_authentication_info(clone_box(info));

        if let Some(subject) = existing {
            context.set_subject(clone_box(subject));
        }
        self.create_subject(Arc::new(context))
    }

    pub fn resolve_session(&self, context: &mut dyn SubjectContext) {
        let session = self.resolve_context_session(context);

        match session {
            Ok(session) => context.set_session(session),
            Err(error) => debug!(
                "Resolved SubjectContext context session is invalid.  Ignoring and creating an anonymous (session-less) Subject instance., error: {:?}",
                error
            ),
        };
    }

    pub fn resolve_context_session(
        &self,
        context: &dyn SubjectContext,
    ) -> Result<Arc<dyn Session>, SessionError> {
        let session_id = self.get_session_id(context);
        self.sessions_security_manager.get_session(&session_id)
    }

    pub fn resolve_principals(&self, context: &mut dyn SubjectContext) {
        let principals = context.resolve_principals();
        if let None = principals {
            debug!(
                "Found remembered PrincipalCollection.  Adding to the context to be used for subject construction by the SubjectFactory."
            );

            let principals = self.get_remembered_identity(context);

            if let Some(principals) = principals {
                if !principals.is_empty() {
                    debug!(
                        "Found remembered PrincipalCollection.  Adding to the context to be used for subject construction by the SubjectFactory."
                    );

                    context.set_principals(principals);
                } else {
                    trace!("No remembered identity found.  Returning original context.");
                }
            }
        }
    }

    pub fn get_remembered_identity(
        &self,
        context: &dyn SubjectContext,
    ) -> Option<Arc<dyn PrincipalCollection>> {
        let rmm = self.get_remember_me_manager();
        return match rmm {
            Some(rmm) => rmm.get_remembered_principals(context),
            None => {
                warn!(
                    "Delegate RememberMeManager instance of type [{}] threw an exception during getRememberedPrincipals().",
                    "RememberMeManager"
                );
                None
            }
        };
    }

    pub fn do_create_subject(&self, context: &dyn SubjectContext) -> Box<dyn Subject> {
        self.get_subject_factory().create_subject(context)
    }

    pub fn save(&self, subject: &dyn Subject) {
        self.get_subject_dao().save(subject)
    }

    pub fn delete(&self, subject: &dyn Subject) {
        self.get_subject_dao().delete(subject)
    }

    pub fn create_session_context(
        &self,
        subject_context: &mut dyn SubjectContext,
    ) -> DefaultSessionContext {
        let mut session_context = DefaultSessionContext::default();

        if !subject_context.is_empty() {
            session_context.put_all(subject_context.values());
        }
        session_context.set_session_id(subject_context.get_session_id().to_owned());

        let host = subject_context.resolve_host();
        if let Some(host) = host {
            session_context.set_host(&host);
        }

        session_context
    }

    pub fn stop_session(&self, subject: &dyn Subject) {
        let session = subject.get_session();
        if let Some(session) = session {
            session.stop();
        }
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
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError> {
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

    fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
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
    T: Authenticator + LogoutAware,
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
                if let Err(err) = self.on_failed_login(token, &error, subject) {
                    info!(
                        "on_failed_login method threw an error.  Logging and propagating original AuthenticationError. error: {:?}",
                        err
                    );
                }
                return Err(error);
            }
        };

        let logged_in = self._create_subject(token, info.as_ref(), Some(subject));

        self.on_successful_login(token, info.as_ref(), logged_in.as_ref());

        Ok(logged_in)
    }

    fn logout(&self, subject: &dyn Subject) -> Result<(), next_web_core::error::BoxError> {
        self.before_logout(subject);
        if let Some(principals) = subject.get_principals() {
            if !principals.is_empty() {
                debug!("Logging out subject with primary principal {}", "none");

                let authc = self
                    .sessions_security_manager
                    .get_object()
                    .get_object()
                    .get_authenticator();

                authc.on_logout(principals.as_ref());
            }
        }

        self.delete(subject);

        self.stop_session(subject);
        Ok(())
    }

    fn create_subject(&self, context: Arc<dyn SubjectContext>) -> Box<dyn Subject> {
        let mut context = DefaultSubjectContext::new(context);

        // self.ensure_security_manager(&mut context);
        self.resolve_session(&mut context);

        self.resolve_principals(&mut context);

        let subject = self.do_create_subject(&context);

        if context.is_session_creation_enabled() {
            self.save(subject.as_ref());
        }

        subject
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
