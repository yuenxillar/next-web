use std::fmt::Display;

use crate::core::authc::authentication_token::AuthenticationToken;

#[derive(Clone, Debug)]
pub struct UsernamePasswordToken {
    username: String,
    password: String,
    remember_me: bool,
    host: Option<String>,
}

impl UsernamePasswordToken {
    pub fn new(
        username: String,
        password: String,
        remember_me: bool,
        host: Option<String>,
    ) -> Self {
        Self {
            username,
            password,
            remember_me,
            host,
        }
    }
}

impl AuthenticationToken for UsernamePasswordToken {
    fn get_principal(&self) -> &crate::core::util::object::Object {
        todo!()
    }

    fn get_credentials(&self) -> Option<&crate::core::util::object::Object> {
        todo!()
    }
}


impl Display for UsernamePasswordToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}