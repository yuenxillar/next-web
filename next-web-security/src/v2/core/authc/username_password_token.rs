use std::fmt::Display;

use crate::core::{
    authc::{
        authentication_token::AuthenticationToken,
        remember_me_authentication_token::RememberMeAuthenticationToken,
    },
    util::object::Object,
};

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
    fn get_principal(&self) -> Object {
        Object::Str(self.username.clone())
    }

    fn get_credentials(&self) -> Option<Object> {
        Some(Object::Str(self.password.clone()))
    }
}

impl RememberMeAuthenticationToken for UsernamePasswordToken {
    fn is_remember_me(&self) -> bool {
        self.remember_me
    }
}

impl Display for UsernamePasswordToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UsernamePasswordToken [username: {}, remember_me: {}, host: {}]",
            self.username,
            self.remember_me,
            self.host.as_deref().unwrap_or("")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::UsernamePasswordToken;
    use crate::core::authc::{
            authentication_token::AuthenticationToken,
            remember_me_authentication_token::RememberMeAuthenticationToken,
        };

    #[test]
    fn the_principal_is_the_user_name_and_the_credentials_the_password() {
        let token = UsernamePasswordToken::new(
            "root".to_string(),
            "secret".to_string(),
            false,
            None,
        );

        assert_eq!(token.get_principal().to_string(), "root");
        assert_eq!(
            token.get_credentials().map(|credentials| credentials.to_string()),
            Some("secret".to_string())
        );
        assert!(!token.is_remember_me());
    }

    #[test]
    fn a_token_can_ask_to_be_remembered() {
        let token = UsernamePasswordToken::new(
            "root".to_string(),
            "secret".to_string(),
            true,
            Some("example.org".to_string()),
        );

        assert!(token.is_remember_me());
        assert_eq!(
            token.to_string(),
            "UsernamePasswordToken [username: root, remember_me: true, host: example.org]"
        );
        assert_eq!(token.get_principal().to_string(), "root");
    }
}