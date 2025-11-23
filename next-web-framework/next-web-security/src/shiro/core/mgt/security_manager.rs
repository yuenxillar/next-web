use std::{fmt::Display, sync::Arc};

#[cfg(feature = "web")]
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};
use next_web_core::{async_trait, error::BoxError};

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_token::AuthenticationToken,
        authenticator::Authenticator,
    },
    authz::authorizer::Authorizer,
    session::mgt::session_manager::SessionManager,
    subject::{subject_context::SubjectContext, Subject},
};
#[cfg(feature = "web")]
use crate::web::subject::web_subject::WebSubject;

#[cfg(not(feature = "web"))]
#[async_trait]
pub trait SecurityManager
where
    Self: Send + Sync,
    Self: Display,
    Self: Authenticator + Authorizer + SessionManager,
{
    async fn login(
        &self,
        subject: &dyn Subject,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn Subject>, AuthenticationError>;

    async fn logout(&self, subject: &dyn Subject) -> Result<(), BoxError>;

    async fn create_subject(&self, context: Arc<dyn SubjectContext>) -> Box<dyn Subject>;
}

#[cfg(feature = "web")]
#[async_trait]
pub trait SecurityManager
where
    Self: Send + Sync,
    Self: Display,
    Self: Authenticator + Authorizer + SessionManager,
{
    async fn login(
        &self,
        subject: &dyn WebSubject,
        authentication_token: &dyn AuthenticationToken,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Subject>, AuthenticationError>;

    async fn logout(
        &self,
        subject: &dyn WebSubject,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;

    async fn create_subject(
        &self,
        context: Arc<dyn SubjectContext>,
        req:  &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Box<dyn WebSubject>;
}
