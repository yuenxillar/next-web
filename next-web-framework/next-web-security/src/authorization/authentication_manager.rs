use crate::core::{authentication::Authentication, authentication_error::AuthenticationError};



pub trait AuthenticationManager:Send + Sync {
    fn authenticate( &self, authentication: &dyn Authentication) -> Result<&dyn Authentication,  AuthenticationError>;
}