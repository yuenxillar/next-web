use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    core::{
        authc::{
            authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        },
        session::{Session, SessionId},
        subject::{
            principal_collection::PrincipalCollection, subject_context::SubjectContext,
            support::default_subject_context::DefaultSubjectContext, Subject,
        },
        util::object::Object,
    },
    web::{
        mgt::web_security_manager::WebSecurityManager,
        subject::web_subject_context::WebSubjectContext,
    },
};

#[derive(Clone)]
pub struct DefaultWebSubjectContext {
    default_subject_context: DefaultSubjectContext,
}

impl DefaultWebSubjectContext {
    const SERVLET_REQUEST: &str =
        stringify!(format!("{}.SERVLET_REQUEST", std::any::type_name::<Self>()));
    const SERVLET_RESPONSE: &str = stringify!(format!(
        "{}.SERVLET_RESPONSE",
        std::any::type_name::<Self>()
    ));

    pub fn new(default_subject_context: DefaultSubjectContext) -> Self {
        Self {
            default_subject_context,
        }
    }
}

impl WebSubjectContext for DefaultWebSubjectContext {}

#[async_trait]
impl SubjectContext for DefaultWebSubjectContext {
    fn get_session_id(&self) -> Option<&SessionId> {
        self.default_subject_context.get_session_id()
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        self.default_subject_context.set_session_id(session_id)
    }

    fn get_subject(&self) -> Option<&dyn Subject> {
        self.default_subject_context.get_subject()
    }

    fn set_subject(&mut self, subject: Box<dyn Subject>) {
        self.default_subject_context.set_subject(subject)
    }

    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        self.default_subject_context.get_principals()
    }

    async fn resolve_security_manager(&self) -> Option<&Arc<dyn WebSecurityManager>> {
        self.default_subject_context
            .resolve_security_manager()
            .await
    }

    async fn resolve_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        self.default_subject_context.resolve_principals().await
    }

    fn set_principals(&mut self, principals: std::sync::Arc<dyn PrincipalCollection>) {
        self.default_subject_context.set_principals(principals)
    }

    fn get_session(&self) -> Option<&Arc<dyn Session>> {
        self.default_subject_context.get_session()
    }

    fn set_session(&mut self, session: std::sync::Arc<dyn Session>) {
        self.default_subject_context.set_session(session)
    }

    fn resolve_session(&self) -> Option<&Arc<dyn Session>> {
        self.default_subject_context.resolve_session()
    }

    fn is_authenticated(&self) -> bool {
        self.default_subject_context.is_authenticated()
    }

    fn set_authenticated(&mut self, authc: bool) {
        self.default_subject_context.set_authenticated(authc)
    }

    fn is_session_creation_enabled(&self) -> bool {
        self.default_subject_context.is_session_creation_enabled()
    }

    fn set_session_creation_enabled(&mut self, enabled: bool) {
        self.default_subject_context
            .set_session_creation_enabled(enabled)
    }

    async fn resolve_authenticated(&self) -> bool {
        self.default_subject_context.resolve_authenticated().await
    }

    fn get_authentication_info(&self) -> Option<&dyn AuthenticationInfo> {
        self.default_subject_context.get_authentication_info()
    }

    fn set_authentication_info(&mut self, info: Box<dyn AuthenticationInfo>) {
        self.default_subject_context.set_authentication_info(info)
    }

    fn get_authentication_token(&self) -> Option<&dyn AuthenticationToken> {
        self.default_subject_context.get_authentication_token()
    }

    fn set_authentication_token(&mut self, token: Box<dyn AuthenticationToken>) {
        self.default_subject_context.set_authentication_token(token)
    }

    fn get_host(&self) -> Option<&str> {
        self.default_subject_context.get_host()
    }

    fn set_host(&mut self, host: String) {
        self.default_subject_context.set_host(host)
    }

    async fn resolve_host(&self) -> Option<String> {
        self.default_subject_context.resolve_host().await
    }

    fn is_empty(&self) -> bool {
        self.default_subject_context.is_empty()
    }

    fn values(&self) -> Vec<(String, Object)> {
        self.default_subject_context.values()
    }
}

impl Default for DefaultWebSubjectContext {
    fn default() -> Self {
        Self {
            default_subject_context: Default::default(),
        }
    }
}
