use crate::core::{authentication::Authentication, authentication_error::AuthenticationError};

pub trait AuthenticationManager: Send + Sync {
    fn authenticate<'a>(
        &self,
        authentication: &'a dyn Authentication,
    ) -> Result<&'a dyn Authentication, AuthenticationError>;
}
