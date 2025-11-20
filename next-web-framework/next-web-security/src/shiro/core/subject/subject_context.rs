use std::sync::Arc;

use next_web_core::async_trait;

use crate::{core::{
    authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken},
    session::{Session, SessionId},
    subject::{Subject, principal_collection::PrincipalCollection},
    util::object::Object,
}, web::mgt::web_security_manager::WebSecurityManager};

#[async_trait]
pub trait SubjectContext
where
    Self: Send + Sync,
{
    fn get_session_id(&self) -> &SessionId;

    fn set_session_id(&mut self, session_id: SessionId);

    fn get_subject(&self) -> Option<&dyn Subject>;

    fn set_subject(&mut self, subject: Box<dyn Subject>);

    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>>;

    async fn resolve_security_manager(&self) -> Option<&Arc<dyn WebSecurityManager>>;

    async fn resolve_principals(&self) -> Option<&Arc<dyn PrincipalCollection>>;

    fn set_principals(&mut self, principals: Arc<dyn PrincipalCollection>);

    fn get_session(&self) -> Option<&Arc<dyn Session>>;

    fn set_session(&mut self, session: Arc<dyn Session>);

    fn resolve_session(&self) -> Option<&Arc<dyn Session>>;

    fn is_authenticated(&self) -> bool;

    fn set_authenticated(&mut self, authc: bool);

    fn is_session_creation_enabled(&self) -> bool;

    fn set_session_creation_enabled(&mut self, enabled: bool);

    async fn resolve_authenticated(&self) -> bool;

    fn get_authentication_info(&self) -> Option<&dyn AuthenticationInfo>;

    fn set_authentication_info(&mut self, info: Box<dyn AuthenticationInfo>);

    fn get_authentication_token(&self) -> Option<&dyn AuthenticationToken>;

    fn set_authentication_token(&mut self, token: Box<dyn AuthenticationToken>);

    fn get_host(&self) -> Option<&str>;

    fn set_host(&mut self, host: String);

    async fn resolve_host(&self) -> Option<String>;

    fn is_empty(&self) -> bool;

    fn values(&self) -> Vec<(String, Object)>;
}
