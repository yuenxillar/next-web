use std::sync::Arc;

use next_web_core::{
    async_trait, error::illegal_state_error::IllegalStateError, traits::required::Required,
};

use crate::{
    core::{
        authc::{
            authentication_error::AuthenticationError, authentication_token::AuthenticationToken,
        },
        authz::authorization_error::AuthorizationError,
        mgt::{
            default_security_manager::DefaultSecurityManager, security_manager::SecurityManager,
        },
        session::{mgt::session_context::SessionContext, Session},
        subject::{
            principal_collection::PrincipalCollection,
            support::delegating_subject::{DelegatingSubject, DelegatingSubjectSupport},
            Subject,
        },
        util::object::Object,
    },
    web::{
        session::mgt::default_web_session_context::DefaultWebSessionContext,
        subject::web_subject::WebSubject,
    },
};

pub const DEFAULT_WEB_DELEGATING_SUBJECT_KEY: &str = "DefaultWebDelegatingSubject";

#[derive(Clone)]
pub struct WebDelegatingSubject<T = DefaultSecurityManager> {
    delegating_subject: DelegatingSubject<T>,
}

impl<T> WebDelegatingSubject<T>
where
    T: SecurityManager + Clone,
    T: 'static,
{
    pub fn new<S>(
        principals: Option<Arc<dyn PrincipalCollection>>,
        authenticated: bool,
        host: Option<String>,
        session: Option<Arc<dyn Session>>,
        security_manager: T,
    ) -> Self {
        let delegating_subject = DelegatingSubject::new(
            principals,
            authenticated,
            host,
            session,
            true,
            security_manager,
        );
        Self { delegating_subject }
    }

    pub fn create_session_context(&self) -> DefaultWebSessionContext {
        let mut wsc = DefaultWebSessionContext::new(Default::default());

        let host = self.delegating_subject.get_host();

        if let Some(host) = host {
            if !host.is_empty() {
                wsc.set_host(host);
            }
        }

        wsc
    }
}

impl WebSubject for WebDelegatingSubject {}

impl<T> DelegatingSubjectSupport for WebDelegatingSubject<T>
where
    T: SecurityManager + Clone,
    T: 'static,
{
    fn is_session_creation_enabled(&self) -> bool {
        self.delegating_subject.is_session_creation_enabled()
    }
}

#[async_trait]
impl<T> Subject for WebDelegatingSubject<T>
where
    T: SecurityManager + Clone,
    T: 'static,
{
    async fn get_principal(&self) -> Option<&Object> {
        self.delegating_subject.get_principal().await
    }

    async fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        self.delegating_subject.get_principals().await
    }

    async fn is_authenticated(&self) -> bool {
        self.delegating_subject.is_authenticated().await
    }

    async fn is_remembered(&self) -> bool {
        self.delegating_subject.is_remembered().await
    }

    async fn is_permitted(&self, permission: &str) -> bool {
        self.delegating_subject.is_permitted(permission).await
    }

    async fn is_permitted_all(&self, permissions: &[&str]) -> bool {
        self.delegating_subject.is_permitted_all(permissions).await
    }

    async fn check_permission(&self, permission: &str) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permission(permission).await
    }

    async fn check_permissions(&self, permissions: &[&str]) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permissions(permissions).await
    }

    async fn has_role(&self, role: &str) -> bool {
        self.delegating_subject.has_role(role).await
    }

    async fn has_all_roles(&self, roles: &[&str]) -> bool {
        self.delegating_subject.has_all_roles(roles).await
    }

    async fn check_role(&self, role: &str) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permission(role).await
    }

    async fn check_roles(&self, roles: &[&str]) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permissions(roles).await
    }

    fn get_session(&self) -> Option<&dyn Session> {
        self.delegating_subject.get_session()
    }

    async fn get_session_or_create(&mut self, create: bool) -> Option<Arc<dyn Session>> {
        self.delegating_subject.get_session_or_create(create).await
    }

    async fn login(&mut self, token: &dyn AuthenticationToken) -> Result<(), AuthenticationError> {
        self.delegating_subject.login(token).await
    }

    async fn logout(&mut self) -> Result<(), next_web_core::error::BoxError> {
        self.delegating_subject.logout().await
    }

    async fn run_as(
        &mut self,
        principals: &Arc<dyn PrincipalCollection>,
    ) -> Result<(), IllegalStateError> {
        self.delegating_subject.run_as(principals).await
    }

    async fn is_run_as(&self) -> bool {
        self.delegating_subject.is_run_as().await
    }

    async fn get_previous_principals(&self) -> Option<Arc<dyn PrincipalCollection>> {
        self.delegating_subject.get_previous_principals().await
    }

    async fn release_run_as(&mut self) -> Option<&dyn PrincipalCollection> {
        self.delegating_subject.release_run_as().await
    }
}

impl Required<DelegatingSubject> for WebDelegatingSubject {
    fn get_object(&self) -> &DelegatingSubject {
        &self.delegating_subject
    }

    fn get_mut_object(&mut self) -> &mut DelegatingSubject {
        &mut self.delegating_subject
    }
}
