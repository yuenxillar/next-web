use std::sync::Arc;

use crate::core::{authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken}, util::object::Object, session::{Session, SessionId}, subject::{principal_collection::PrincipalCollection, Subject}};



pub trait SubjectContext
where 
Self: Send + Sync,
{
    fn get_session_id(&self) -> & SessionId;

    fn set_session_id(&mut self, session_id: SessionId);

    fn get_subject(&self) -> &dyn Subject;

    fn set_subject(&mut self, subject: Box<dyn Subject>);

    fn get_principals(&self) -> &dyn PrincipalCollection;

    fn resolve_principals(&mut self) -> Option<Arc<dyn PrincipalCollection>>;

    fn set_principals(&mut self, principals:  Arc<dyn PrincipalCollection>);

    fn get_session(&self) -> &dyn Session;

    fn set_session(&mut self, session: Arc<dyn Session>);

    fn resolve_session(&mut self) -> Box<dyn Session>;

    fn is_authenticated(&self) -> bool;

    fn set_authenticated(&mut self, authc: bool);

    fn is_session_creation_enabled(&self) -> bool;

    fn set_session_creation_enabled(&mut self, enabled: bool);

    fn resolve_authenticated(&mut self) -> bool;

    fn get_authentication_info(&self) -> &dyn AuthenticationInfo;

    fn set_authentication_info(&mut self, info: Box<dyn AuthenticationInfo>);

    fn get_authentication_token(&self) -> &dyn AuthenticationToken;

    fn set_authentication_token(&mut self, token: Box<dyn AuthenticationToken>);

    fn get_host(&self) -> Option<&str>;

    fn set_host(&mut self, host: String);

    fn resolve_host(&mut self) -> Option<String>;

    fn is_empty(&self) -> bool;

    fn values(&self) -> Vec<(String, Object)>;

}