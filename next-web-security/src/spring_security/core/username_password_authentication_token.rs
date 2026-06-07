use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::core::{
    Authentication, credentials_container::CredentialsContainer,
    granted_authority::GrantedAuthority,
};

#[derive(Clone, Default)]
pub struct UsernamePasswordAuthenticationToken {
    principal: Option<String>,
    credentials: Option<String>,
    details: Option<AnyValue>,
    authenticated: bool,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl UsernamePasswordAuthenticationToken {
    pub fn unauthenticated(
        principal: Option<String>,
        credentials: Option<String>,
    ) -> Self {
        Self {
            principal,
            credentials,
            details: None,
            authenticated: false,
            authorities: Vec::new(),
        }
    }

    pub fn authenticated(
        principal: impl Into<String>,
        credentials: Option<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal: Some(principal.into()),
            credentials,
            details: None,
            authenticated: true,
            authorities,
        }
    }

    pub fn set_details(&mut self, details: Option<String>) {
        self.details = details.map(AnyValue::from);
    }

    pub fn set_details_value(&mut self, details: Option<AnyValue>) {
        self.details = details;
    }

    pub fn authorities_objects(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

impl Authentication for UsernamePasswordAuthenticationToken {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn authentication_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn get_credentials(&self) -> Option<String> {
        self.credentials.clone()
    }

    fn get_details_ref(&self) -> Option<&AnyValue> {
        self.details.as_ref()
    }

    fn get_principal(&self) -> Option<String> {
        self.principal.clone()
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), &'static str> {
        if is_authenticated {
            return Err(
                "Cannot set this token to trusted - use constructor which takes a GrantedAuthority list instead",
            );
        }
        self.authenticated = false;
        Ok(())
    }

    fn authorities(&self) -> Vec<String> {
        let mut authorities = Vec::with_capacity(self.authorities.len());
        for authority in &self.authorities {
            if let Some(authority) = block_on(authority.get_authority()) {
                authorities.push(authority);
            }
        }
        authorities
    }
}

impl CredentialsContainer for UsernamePasswordAuthenticationToken {
    fn erase_credentials(&mut self) {
        self.credentials = None;
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    futures::executor::block_on(future)
}

#[cfg(test)]
mod tests {
    use crate::core::{
        authority_utils::AuthorityUtils, Authentication,
        credentials_container::CredentialsContainer,
    };

    use super::UsernamePasswordAuthenticationToken;

    #[test]
    fn authenticated_factory_marks_token_authenticated() {
        let token = UsernamePasswordAuthenticationToken::authenticated(
            "alice",
            Some(String::from("secret")),
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        assert!(token.is_authenticated());
        assert_eq!(token.get_name(), "alice");
        assert_eq!(token.authorities(), vec!["ROLE_USER".to_string()]);
    }

    #[test]
    fn erase_credentials_clears_password() {
        let mut token = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );

        token.erase_credentials();
        assert_eq!(token.get_credentials(), None);
    }
}
