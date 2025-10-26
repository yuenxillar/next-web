use std::{fmt::Display, sync::Arc};

use crate::core::{
    authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken}, session::{Session, SessionId}, subject::{
        principal_collection::PrincipalCollection, subject_context::SubjectContext, Subject
    }
};

#[derive(Clone)]
pub struct DefaultSubjectContext {}


impl DefaultSubjectContext {
    pub fn new(context: Arc<dyn SubjectContext>) -> Self {
        Self {}
    }
}

impl SubjectContext for DefaultSubjectContext {
    fn get_session_id(&self) -> &SessionId {
        todo!()
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        todo!()
    }

    fn get_subject(&self) -> &dyn Subject {
        todo!()
    }

    fn set_subject(&mut self, subject: Box<dyn Subject>) {
        todo!()
    }

    fn get_principals(&self) -> &dyn PrincipalCollection {
        todo!()
    }

    fn resolve_principals(&mut self) -> Option<Arc<dyn PrincipalCollection>>{
        todo!()
    }

    fn set_principals(&mut self, principals: Arc<dyn PrincipalCollection>) {
        todo!()
    }

    fn get_session(&self) -> &dyn Session {
        todo!()
    }

    fn set_session(&mut self, session: Arc<dyn Session>) {
        todo!()
    }

    fn resolve_session(&mut self) -> Box<dyn Session> {
        todo!()
    }

    fn is_authenticated(&self) -> bool {
        todo!()
    }

    fn set_authenticated(&mut self, authc: bool) {
        todo!()
    }

    fn is_session_creation_enabled(&self) -> bool {
        todo!()
    }

    fn set_session_creation_enabled(&mut self, enabled: bool) {
        todo!()
    }

    fn resolve_authenticated(&mut self) -> bool {
        todo!()
    }

    fn get_authentication_info(
        &self,
    ) -> &dyn AuthenticationInfo {
        todo!()
    }

    fn set_authentication_info(
        &mut self,
        info: Box<dyn AuthenticationInfo>,
    ) {
        todo!()
    }

    fn get_authentication_token(
        &self,
    ) -> &dyn AuthenticationToken {
        todo!()
    }

    fn set_authentication_token(
        &mut self,
        token: Box<dyn AuthenticationToken>,
    ) {
        todo!()
    }

    fn get_host(&self) -> Option<&str> {
        todo!()
    }

    fn set_host(&mut self, host: String) {
        todo!()
    }

    fn resolve_host(&mut self) -> Option<String> {
        todo!()
    }
    
    fn is_empty(&self) -> bool {
        todo!()
    }
    
    fn values(&self) -> Vec<(String, crate::core::util::object::Object)> {
        todo!()
    }
}


impl Display for DefaultSubjectContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultSubjectContext")
    }
}

impl Default for DefaultSubjectContext {
    fn default() -> Self {
        Self {}
    }
}
