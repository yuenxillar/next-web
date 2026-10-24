use std::sync::Arc;

use crate::core::{Authentication, AuthenticationError};

pub trait AuthenticationEventPublisher
where
    Self: Send + Sync,
{
    fn publish_authentication_success(&self, authentication: Arc<dyn Authentication>);

    fn publish_authentication_failure(
        &self,
        error: AuthenticationError,
        authentication: Arc<dyn Authentication>,
    );
}

#[derive(Clone, Default)]
pub struct NullAuthenticationEventPublisher;

#[allow(unused_variables)]
impl AuthenticationEventPublisher for NullAuthenticationEventPublisher {
    fn publish_authentication_success(&self, authentication: Arc<dyn Authentication>) {}

    fn publish_authentication_failure(
        &self,
        error: AuthenticationError,
        authentication: Arc<dyn Authentication>,
    ) {
    }
}
