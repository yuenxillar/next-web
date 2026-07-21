use std::sync::Arc;

use next_web_core::{anys::any_value::AnyValue, error::BoxError};

use crate::{
    core::{
        credentials_container::CredentialsContainer, granted_authority::GrantedAuthority,
        Authentication,
    },
    web::authentication::AuthPrincipal,
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
        principal: impl Into<Option<String>>,
        credentials: impl Into<Option<String>>,
    ) -> Self {
        Self {
            principal: principal.into(),
            credentials: credentials.into(),
            details: None,
            authenticated: false,
            authorities: Vec::new(),
        }
    }

    pub fn authenticated(
        principal: impl Into<String>,
        credentials: impl Into<Option<String>>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal: Some(principal.into()),
            credentials: credentials.into(),
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
    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.credentials
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.details
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        if is_authenticated {
            return Err(
                "Cannot set this token to trusted - use constructor which takes a GrantedAuthority list instead".into()
            );
        }
        self.authenticated = false;

        Ok(())
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        // let mut authorities = Vec::with_capacity(self.authorities.len());
        // for authority in &self.authorities {
        //     if let Some(authority) = authority.authority() {
        //         authorities.push(authority.to_string());
        //     }
        // }
        todo!()
    }
}

impl CredentialsContainer for UsernamePasswordAuthenticationToken {
    fn erase_credentials(&self) {
        // self.credentials = None;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{
        authority_utils::AuthorityUtils, credentials_container::CredentialsContainer,
        Authentication,
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
