use std::{any::Any, fmt::Display, sync::Arc};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{
        authc::{
            authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
            authentication_token::AuthenticationToken, authenticator::Authenticator,
            pam::modular_realm_authenticator::ModularRealmAuthenticator,
        },
        authz::{
            authorization_error::AuthorizationError, authorizer::Authorizer,
            modular_realm_authorizer::ModularRealmAuthorizer, permission::Permission,
        },
        cache::default_cache_manager::DefaultCacheManager,
        event::support::default_event_bus::DefaultEventBus,
        mgt::{
            default_security_manager::DefaultSecurityManager,
            default_subject_dao::DefaultSubjectDAO, security_manager::SecurityManager,
        },
        realm::simple_account_realm::SimpleAccountRealm,
        session::{
            mgt::{session_context::SessionContext, session_manager::SessionManager},
            Session, SessionError, SessionId,
        },
        subject::{
            principal_collection::PrincipalCollection, subject_context::SubjectContext, Subject,
        },
    },
    web::{
        mgt::{
            cookie_remember_me_manager::CookieRememberMeManager,
            default_web_session_storage_evaluator::DefaultWebSessionStorageEvaluator,
            default_web_subject_factory::DefaultWebSubjectFactory,
            web_security_manager::WebSecurityManager,
        },
        session::mgt::{
            default_web_session_context::DefaultWebSessionContext,
            default_web_session_manager::DefaultWebSessionManager,
        },
        subject::web_subject::WebSubject,
    },
};

// D = DefaultSubjectDAO,
// F = DefaultSubjectFactory,
// S = DefaultSessionManager,
// A = ModularRealmAuthorizer,
// T = ModularRealmAuthenticator,
// R = SimpleAccountRealm,
// C = DefaultCacheManager,
// B = DefaultEventBus,

#[derive(Clone)]
pub struct DefaultWebSecurityManager {
    default_security_manager: DefaultSecurityManager<
        DefaultSubjectDAO,
        DefaultWebSubjectFactory,
        DefaultWebSessionManager,
        ModularRealmAuthorizer,
        ModularRealmAuthenticator,
        SimpleAccountRealm,
        DefaultCacheManager,
        DefaultEventBus,
    >,
}

impl DefaultWebSecurityManager {
    fn init(&mut self, key: Option<Vec<u8>>) {
        let mut web_evaluator = DefaultWebSessionStorageEvaluator::default();

        self.default_security_manager
            .set_subject_factory(DefaultWebSubjectFactory::default());
        let cookie_remember_me_manager = if let Some(key) = key {
            CookieRememberMeManager::new(key)
        } else {
            CookieRememberMeManager::default()
        };
        self.default_security_manager
            .set_remember_me_manager(cookie_remember_me_manager);

        self.set_session_manager(DefaultWebSessionManager::default());
        web_evaluator.set_session_manager(
            self.default_security_manager
                .sessions_security_manager
                .get_session_manager()
                .clone(),
        );

        self.default_security_manager
            .subject_dao
            .set_session_storage_evaluator(web_evaluator);
    }

    pub fn set_subject_dao(&mut self, subject_dao: DefaultSubjectDAO) {
        self.default_security_manager.set_subject_dao(subject_dao);
        self.apply_session_manager_to_session_storage_evaluator_if_possible();
    }

    fn apply_session_manager_to_session_storage_evaluator_if_possible(&mut self) {
        let manager = self
            .default_security_manager
            .sessions_security_manager
            .get_session_manager()
            .clone();
        let subject_dao = self.default_security_manager.get_mut_subject_dao();
        let evaluator = subject_dao.get_mut_session_storage_evaluator();
        if let Some(evaluator) =
            (evaluator as &mut dyn Any).downcast_mut::<DefaultWebSessionStorageEvaluator>()
        {
            evaluator.set_session_manager(manager);
        }
    }

    pub fn set_session_manager(&mut self, session_manager: DefaultWebSessionManager) {
        self.default_security_manager
            .sessions_security_manager
            .set_session_manager(session_manager);
    }

    pub async fn create_session_context(
        &self,
        subject_context: &mut dyn SubjectContext,
    ) -> DefaultWebSessionContext {
        let session_context = self
            .default_security_manager
            .create_session_context(subject_context)
            .await;

        DefaultWebSessionContext::new(session_context)
    }

    pub fn get_session_key<'a>(&'a self, context: &'a dyn SubjectContext) -> Option<&'a SessionId> {
        context.get_session_id()
    }

    pub fn before_logout(
        &self,
        subject: &dyn WebSubject,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self.default_security_manager
            .before_logout(subject, req, resp);
    }

    pub fn get_execution_filters(&self) {}
}

impl WebSecurityManager for DefaultWebSecurityManager {
    fn is_http_session_mode(&self) -> bool {
        true
    }
}

#[async_trait]
impl SecurityManager for DefaultWebSecurityManager {
    async fn login(
        &self,
        subject: &dyn WebSubject,
        authentication_token: &dyn AuthenticationToken,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Subject>, AuthenticationError> {
        self.default_security_manager
            .login(subject, authentication_token, req, resp)
            .await
    }

    async fn logout(
        &self,
        subject: &dyn WebSubject,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), next_web_core::error::BoxError> {
        self.default_security_manager
            .logout(subject, req, resp)
            .await
    }

    async fn create_subject(
        &self,
        context: Arc<dyn SubjectContext>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Box<dyn WebSubject> {
        self.default_security_manager
            .create_subject(context, req, resp)
            .await
    }
}

impl Authenticator for DefaultWebSecurityManager {
    fn authenticate(
        &self,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError> {
        self.default_security_manager
            .authenticate(authentication_token)
    }
}

impl Authorizer for DefaultWebSecurityManager {
    fn is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        self.default_security_manager
            .is_permitted(principal, permission)
    }

    fn is_permitted_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        self.default_security_manager
            .is_permitted_from_str(principal, permission)
    }

    fn is_permitted_from_str_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        self.default_security_manager
            .is_permitted_from_str_list(principal, permissions)
    }

    fn is_permitted_from_permission_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        self.default_security_manager
            .is_permitted_from_permission_list(principal, permissions)
    }

    fn is_permitted_all(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        self.default_security_manager
            .is_permitted_all(principal, permissions)
    }

    fn is_permitted_all_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        self.default_security_manager
            .is_permitted_all_from_str(principal, permissions)
    }

    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        self.default_security_manager
            .check_permission(principal, permission)
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        self.default_security_manager
            .check_permission_from_str(principal, permission)
    }

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        self.default_security_manager
            .check_permissions(principals, permissions)
    }

    fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.default_security_manager
            .check_permissions_from_str(principals, permissions)
    }

    fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool {
        self.default_security_manager
            .has_role(principal, role_identifier)
    }

    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        self.default_security_manager
            .has_roles(principal, role_identifiers)
    }

    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        self.default_security_manager
            .has_all_roles(principal, role_identifiers)
    }

    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        self.default_security_manager.check_role(principal, role)
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.default_security_manager.check_roles(principal, roles)
    }
}

#[async_trait]
impl SessionManager for DefaultWebSecurityManager {
    async fn start(
        &self,
        context: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Session>, AuthorizationError> {
        self.default_security_manager
            .start(context, req, resp)
            .await
    }

    async fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
        self.default_security_manager.get_session(id).await
    }
}

impl Default for DefaultWebSecurityManager {
    fn default() -> Self {
        let mut manager = Self {
            default_security_manager: Default::default(),
        };

        manager.init(None);

        manager
    }
}

impl Display for DefaultWebSecurityManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultWebSecurityManager")
    }
}
