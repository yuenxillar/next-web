use std::sync::Arc;

use axum::{extract::Request, response::Response};
use next_web_core::{error::illegal_state_error::IllegalStateError, traits::required::Required};

use crate::{
    core::{
        authc::{
            authentication_error::AuthenticationError, authentication_token::AuthenticationToken,
        },
        authz::authorization_error::AuthorizationError,
        mgt::{default_security_manager::DefaultSecurityManager, security_manager::SecurityManager},
        util::object::Object,
        session::{mgt::session_context::SessionContext, Session},
        subject::{
            principal_collection::PrincipalCollection, support::delegating_subject::{DelegatingSubject, DelegatingSubjectSupport}, Subject
        },
    },
    web::{
        session::mgt::default_web_session_context::DefaultWebSessionContext,
        subject::web_subject::WebSubject,
    },
};

#[derive(Clone)]
pub struct WebDelegatingSubject<T = DefaultSecurityManager>
{
    delegating_subject: DelegatingSubject<T>,
}

impl<T> WebDelegatingSubject<T> 
where 
T: SecurityManager + Clone,
T: 'static
{
    pub fn new<S>(
        principals: Option<Arc<dyn PrincipalCollection>>,
        authenticated: bool,
        host: Option<String>,
        session: Option<Box<dyn Session>>,
        security_manager: T
    ) -> Self  {
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
}

impl WebSubject for WebDelegatingSubject {
    fn request(&self) -> &mut Request {
        todo!()
    }

    fn response(&self) -> &mut Response {
        todo!()
    }
}

impl DelegatingSubjectSupport for WebDelegatingSubject {
    fn is_session_creation_enabled(&self) -> bool {
        self.delegating_subject.is_session_creation_enabled()
    }

    fn create_session_context(
        &self,
    ) -> impl crate::core::session::mgt::session_context::SessionContext + 'static {
        let mut wsc = DefaultWebSessionContext::default();

        let host = self.delegating_subject.get_host();

        if let Some(host) = host {
            if !host.is_empty() {
                wsc.set_host(host);
            }
        }

        wsc
    }
}

impl<T>  Subject for WebDelegatingSubject<T>  
where 
T: SecurityManager + Clone,
T: 'static
{
    fn get_principal(&self) -> Option<&Object> {
        self.delegating_subject.get_principal()
    }

    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        self.delegating_subject.get_principals()
    }

    fn is_authenticated(&self) -> bool {
        self.delegating_subject.is_authenticated()
    }

    fn is_remembered(&self) -> bool {
        self.delegating_subject.is_remembered()
    }

    fn is_permitted(&self, permission: &str) -> bool {
        self.delegating_subject.is_permitted(permission)
    }

    fn is_permitted_all(&self, permissions: &[&str]) -> bool {
        self.delegating_subject.is_permitted_all(permissions)
    }

    fn check_permission(&self, permission: &str) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permission(permission)
    }

    fn check_permissions(&self, permissions: &[&str]) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permissions(permissions)
    }

    fn has_role(&self, role: &str) -> bool {
        self.delegating_subject.has_role(role)
    }

    fn has_all_roles(&self, roles: &[&str]) -> bool {
        self.delegating_subject.has_all_roles(roles)
    }

    fn check_role(&self, role: &str) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permission(role)
    }

    fn check_roles(&self, roles: &[&str]) -> Result<(), AuthorizationError> {
        self.delegating_subject.check_permissions(roles)
    }

    fn get_session(&self) -> Option<&dyn Session> {
        self.delegating_subject.get_session()
    }

    fn get_session_or_create(&mut self, create: bool) -> Option<&mut Box<dyn Session>> {
        self.delegating_subject.get_session_or_create(create)
    }

    fn login(&mut self, token: &dyn AuthenticationToken) -> Result<(), AuthenticationError> {
        self.delegating_subject.login(token)
    }

    fn logout(&mut self) -> Result<(), next_web_core::error::BoxError> {
        self.delegating_subject.logout()
    }

    fn run_as(&mut self, principals: &Arc<dyn PrincipalCollection>) -> Result<(), IllegalStateError> {
        self.delegating_subject.run_as(principals)
    }

    fn is_run_as(&self) -> bool {
        self.delegating_subject.is_run_as()
    }

    fn get_previous_principals(&self) -> Option<Arc<dyn PrincipalCollection>> {
        self.delegating_subject.get_previous_principals()
    }

    fn release_run_as(&mut self) -> Option<&dyn PrincipalCollection> {
        self.delegating_subject.release_run_as()
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
