use crate::core::{
    authc::{
        authentication_token::AuthenticationToken,
        host_authentication_token::HostAuthenticationToken,
    },
    util::object::Object,
};

#[derive(Clone)]
pub struct BearerToken {
    token: String,
    host: Option<String>,
}

impl BearerToken {
    pub fn new(token: String, host: Option<String>) -> Self {
        Self { token, host }
    }

    pub fn get_token(&self) -> &str {
        &self.token
    }
}

impl AuthenticationToken for BearerToken {
    fn get_principal(&self) -> Object {
        Object::Str(self.token.clone())
    }

    fn get_credentials(&self) -> Option<Object> {
        Some(Object::Str(self.token.clone()))
    }
}

impl HostAuthenticationToken for BearerToken {
    fn get_host(&self) -> Option<&str> {
        self.host.as_deref()
    }
}

impl std::fmt::Display for BearerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let visible_prefix = self.token.chars().take(4).collect::<String>();
        let visible_suffix = self
            .token
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        let masked = if self.token.chars().count() <= 8 {
            "***".to_string()
        } else {
            format!("{visible_prefix}***{visible_suffix}")
        };
        write!(
            f,
            "BearerToken [token: {}, host: {}]",
            masked,
            self.host.as_deref().unwrap_or("")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::BearerToken;
    use crate::core::{
        authc::{
            authentication_token::AuthenticationToken,
            host_authentication_token::HostAuthenticationToken,
        },
    };

    #[test]
    fn the_token_is_both_the_principal_and_the_credential() {
        let token = BearerToken::new("a-bearer-token".to_string(), None);

        assert_eq!(token.get_token(), "a-bearer-token");
        assert_eq!(token.get_principal().to_string(), "a-bearer-token");
        assert_eq!(
            token.get_credentials().map(|credentials| credentials.to_string()),
            Some("a-bearer-token".to_string())
        );
        assert_eq!(token.get_host(), None);
    }

    #[test]
    fn the_host_of_the_token_is_readable() {
        let token = BearerToken::new("a-bearer-token".to_string(), Some("example.org".to_string()));

        assert_eq!(token.get_host(), Some("example.org"));
    }
}