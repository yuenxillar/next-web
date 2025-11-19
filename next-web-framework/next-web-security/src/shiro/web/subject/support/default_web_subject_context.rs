use crate::core::{
    authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken},
    session::{Session, SessionId},
    subject::{
        Subject, principal_collection::PrincipalCollection, subject_context::SubjectContext, support::default_subject_context::DefaultSubjectContext
    },
    util::object::Object,
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

impl SubjectContext for DefaultWebSubjectContext {
    fn get_session_id(&self) -> &SessionId {
        self.default_subject_context.get_session_id()
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        self.default_subject_context.set_session_id(session_id)
    }

    fn get_subject(&self) -> &dyn Subject {
        self.default_subject_context.get_subject()
    }

    fn set_subject(&mut self, subject: Box<dyn Subject>) {
        self.default_subject_context.set_subject(subject)
    }

    fn get_principals(&self) -> &dyn PrincipalCollection {
        self.default_subject_context.get_principals()
    }

    fn resolve_principals(&mut self) -> Option<std::sync::Arc<dyn PrincipalCollection>> {
        self.default_subject_context.resolve_principals()
    }

    fn set_principals(&mut self, principals: std::sync::Arc<dyn PrincipalCollection>) {
        self.default_subject_context.set_principals(principals)
    }

    fn get_session(&self) -> &dyn Session {
        self.default_subject_context.get_session()
    }

    fn set_session(&mut self, session: std::sync::Arc<dyn Session>) {
        self.default_subject_context.set_session(session)
    }

    fn resolve_session(&mut self) -> Box<dyn Session> {
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
        self.default_subject_context.set_session_creation_enabled(enabled)
    }

    fn resolve_authenticated(&mut self) -> bool {
        self.default_subject_context.resolve_authenticated()
    }

    fn get_authentication_info(&self) -> &dyn AuthenticationInfo {
       self.default_subject_context.get_authentication_info()
    }

    fn set_authentication_info(&mut self, info: Box<dyn AuthenticationInfo>) {
        self.default_subject_context.set_authentication_info(info)
    }

    fn get_authentication_token(&self) -> &dyn AuthenticationToken {
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

    fn resolve_host(&mut self) -> Option<String> {
        self.default_subject_context.resolve_host()
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
