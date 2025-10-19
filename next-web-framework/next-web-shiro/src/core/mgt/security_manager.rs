use crate::core::{
    authc::{authentication_error::AuthenticationError, authentication_token::AuthenticationToken, authenticator::Authenticator},
    authz::authorizer::Authorizer,
    session::mgt::session_manager::SessionManager,
    subject::{subject_context::SubjectContext, Subject},
};

pub trait SecurityManager
where
    Self: Send + Sync,
    Self: Authenticator + Authorizer + SessionManager,
{
    fn login(
        &self,
        subject: &dyn Subject,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn Subject>, AuthenticationError>;

    fn logout(&self, subject: &dyn Subject);

    fn create_subject(&self, context: &dyn SubjectContext) -> &dyn Subject;
}
