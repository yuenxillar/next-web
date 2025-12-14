use std::sync::Arc;

#[cfg(feature = "web")]
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
        authentication_token::AuthenticationToken,
    },
    subject::principal_collection::PrincipalCollection,
};
#[cfg(feature = "web")]
use crate::web::subject::{web_subject::WebSubject, web_subject_context::WebSubjectContext};

#[cfg(not(feature = "web"))]
use crate::core::subject::{subject_context::SubjectContext, Subject};

#[cfg(feature = "web")]
pub trait RememberMeManager
where
    Self: Send + Sync,
{
    fn on_successful_login(
        &self,
        subject: &dyn WebSubject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );

    fn on_failed_login(
        &self,
        subject: &dyn WebSubject,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );

    fn on_logout(
        &self,
        subject: &dyn WebSubject,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );

    fn get_remembered_principals(
        &self,
        subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn PrincipalCollection>>;

    fn forget_identity(
        &self,
        subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );
}

#[cfg(not(feature = "web"))]
pub trait RememberMeManager
where
    Self: Send + Sync,
{
    fn get_remembered_principals(
        &self,
        subject_context: &dyn SubjectContext,
    ) -> Option<Arc<dyn PrincipalCollection>>;

    fn forget_identity(&self, subject_context: &dyn SubjectContext);

    fn on_successful_login(
        &self,
        subject: &mut dyn Subject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    );

    fn on_failed_login(
        &self,
        subject: &mut dyn Subject,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
    );

    fn on_logout(&self, subject: &dyn Subject);
}

#[cfg(not(feature = "web"))]
#[allow(unused_variables)]
impl RememberMeManager for () {
    fn on_successful_login(
        &self,
        subject: &mut dyn Subject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) {
    }

    fn forget_identity(&self, subject_context: &dyn SubjectContext) {}

    fn on_failed_login(
        &self,
        subject: &mut dyn Subject,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
    ) {
    }

    fn on_logout(&self, subject: &mut dyn Subject) {}

    fn get_remembered_principals(
        &self,
        subject_context: &mut dyn SubjectContext,
    ) -> Option<Arc<dyn PrincipalCollection>> {
        None
    }
}

#[cfg(feature = "web")]
#[allow(unused_variables)]
impl RememberMeManager for () {
    fn on_successful_login(
        &self,
        subject: &dyn WebSubject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
    }

    fn forget_identity(
        &self,
        subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
    }

    fn on_failed_login(
        &self,
        subject: &dyn WebSubject,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
    }

    fn on_logout(
        &self,
        subject: &dyn WebSubject,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
    }

    fn get_remembered_principals(
        &self,
        subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn PrincipalCollection>> {
        None
    }
}
