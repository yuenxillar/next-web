use std::{fmt::Display, sync::Arc};

use next_web_core::{async_trait, error::BoxError};

use crate::core::{
    authc::{authentication_error::AuthenticationError, authentication_token::AuthenticationToken, authenticator::Authenticator},
    authz::authorizer::Authorizer,
    session::mgt::session_manager::SessionManager,
    subject::{subject_context::SubjectContext, Subject},
};

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

    fn create_subject(&self, context: Arc<dyn SubjectContext>) -> Box<dyn Subject>;
}