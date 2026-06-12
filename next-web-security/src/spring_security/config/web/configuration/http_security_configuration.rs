use std::sync::Arc;

use crate::crypto::password::PasswordEncoder;

pub struct HttpSecurityConfiguration {}

impl HttpSecurityConfiguration {}

struct DefaultPasswordEncoderAuthenticationManagerBuilder {
    password_encoder: Arc<dyn PasswordEncoder>,
}

impl DefaultPasswordEncoderAuthenticationManagerBuilder {
    pub fn new(default_password_encoder: Arc<dyn PasswordEncoder>) -> Self {
        Self {
            password_encoder: default_password_encoder,
        }
    }
}
