use std::sync::Arc;

use crate::core::{
    Authentication, granted_authority::GrantedAuthority,
};

#[derive(Clone, Default)]
pub struct TestingAuthenticationToken {
    principal: String,
    credentials: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl TestingAuthenticationToken {
    pub fn new(
        principal: impl Into<String>,
        credentials: Option<String>,
    ) -> Self {
        Self {
            principal: principal.into(),
            credentials,
            authorities: Vec::new(),
        }
    }

    pub fn with_authorities(
        principal: impl Into<String>,
        credentials: Option<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal: principal.into(),
            credentials,
            authorities,
        }
    }
}

impl Authentication for TestingAuthenticationToken {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn authentication_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn get_credentials(&self) -> Option<String> {
        self.credentials.clone()
    }

    fn get_principal(&self) -> Option<String> {
        Some(self.principal.clone())
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn authorities(&self) -> Vec<String> {
        self.authorities
            .iter()
            .filter_map(|authority| futures::executor::block_on(authority.get_authority()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{Authentication, authority_utils::AuthorityUtils};

    use super::TestingAuthenticationToken;

    #[test]
    fn testing_authentication_token_is_authenticated() {
        let token = TestingAuthenticationToken::with_authorities(
            "alice",
            Some(String::from("secret")),
            AuthorityUtils::create_authority_list(["ROLE_TEST"]),
        );

        assert!(token.is_authenticated());
        assert_eq!(token.get_name(), "alice");
        assert_eq!(token.authorities(), vec![String::from("ROLE_TEST")]);
    }
}
