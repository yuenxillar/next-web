use crate::crypto::password::password_encoder::PasswordEncoder;
use bcrypt::{hash, verify, DEFAULT_COST};
use next_web_core::error::BoxError;

#[derive(Clone)]
pub struct BCryptPasswordEncoder;

impl PasswordEncoder for BCryptPasswordEncoder {
    fn encode(&self, raw_password: &str) -> Result<String, BoxError> {
        hash(raw_password, DEFAULT_COST).map_err(Into::into)
    }

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        verify(raw_password, encoded_password).unwrap_or(false)
    }
}