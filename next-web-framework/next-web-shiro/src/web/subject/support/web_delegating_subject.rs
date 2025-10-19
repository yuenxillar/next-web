use axum::{extract::Request, response::Response};
use next_web_core::traits::required::Required;

use crate::{
    core::{object::Object, subject::{principal_collection::PrincipalCollection, support::delegating_subject::DelegatingSubject, Subject}},
    web::subject::web_subject::WebSubject,
};

#[derive(Clone)]
pub struct WebDelegatingSubject
where
    Self: Required<DelegatingSubject>, {}

impl WebSubject for WebDelegatingSubject {
    fn request(&self) -> &mut Request {
        todo!()
    }

    fn response(&self) -> &mut Response {
        todo!()
    }
}

impl Subject for WebDelegatingSubject {
    fn get_principal(&self) -> Option<& Object> {
        todo!()
    }

    fn get_principals(&self) -> Option<&dyn PrincipalCollection> {
        todo!()
    }

    fn is_authenticated(&self) -> bool {
        todo!()
    }

    fn is_remembered(&self) -> bool {
        todo!()
    }

    fn is_permitted(&self, permission: &str) -> bool {
        todo!()
    }

    fn is_permitted_all(&self, permissions: &[&str]) -> bool {
        todo!()
    }

    fn check_permission(
        &self,
        permission: &str,
    ) -> Result<(), crate::core::authz::authorization_error::AuthorizationError> {
        todo!()
    }

    fn check_permissions(
        &self,
        permissions: &[&str],
    ) -> Result<(), crate::core::authz::authorization_error::AuthorizationError> {
        todo!()
    }

    fn has_role(&self, role: &str) -> bool {
        todo!()
    }

    fn has_all_roles(&self, roles: &[&str]) -> bool {
        todo!()
    }

    fn check_role(
        &self,
        role: &str,
    ) -> Result<(), crate::core::authz::authorization_error::AuthorizationError> {
        todo!()
    }

    fn check_roles(
        &self,
        roles: &[&str],
    ) -> Result<(), crate::core::authz::authorization_error::AuthorizationError> {
        todo!()
    }

    fn get_session(&self) -> Option<&dyn crate::core::session::Session> {
        todo!()
    }

    fn get_session_or_create(&self, create: bool) -> Option<&dyn crate::core::session::Session> {
        todo!()
    }

    fn login(
        &mut self,
        token: &dyn crate::core::authc::authentication_token::AuthenticationToken,
    ) -> Result<(), crate::core::authc::authentication_error::AuthenticationError> {
        todo!()
    }

    fn logout(&mut self) {
        todo!()
    }

    fn run_as(&mut self, principals: Vec<Object>) -> Result<(), String> {
        todo!()
    }

    fn is_run_as(&self) -> bool {
        todo!()
    }

    fn get_previous_principals(&self) -> Option<Vec<Object>> {
        todo!()
    }

    fn release_run_as(&mut self) -> Option<Vec<Object>> {
        todo!()
    }
}

impl Required<DelegatingSubject> for WebDelegatingSubject {
    fn get_object(&self) -> &DelegatingSubject {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut DelegatingSubject {
        todo!()
    }
}
