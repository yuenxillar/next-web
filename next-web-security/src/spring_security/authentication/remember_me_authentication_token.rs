use std::sync::Arc;

use crate::core::{
    authentication::Authentication, granted_authority::GrantedAuthority,
};

#[derive(Clone, Default)]
pub struct RememberMeAuthenticationToken {
    principal: String,
    key_hash: i32,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl RememberMeAuthenticationToken {
    pub fn new(
        key: impl AsRef<str>,
        principal: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let principal = principal.into();
        assert!(
            !key.as_ref().trim().is_empty(),
            "Cannot pass null or empty values to constructor"
        );
        assert!(
            !principal.trim().is_empty(),
            "Cannot pass null or empty values to constructor"
        );

        Self {
            principal,
            key_hash: java_string_hash(key.as_ref()),
            authorities,
        }
    }

    pub fn key_hash(&self) -> i32 {
        self.key_hash
    }
}

impl Authentication for RememberMeAuthenticationToken {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn authentication_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn get_credentials(&self) -> Option<String> {
        Some(String::new())
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

    fn is_remember_me(&self) -> bool {
        true
    }
}

fn java_string_hash(value: &str) -> i32 {
    value
        .chars()
        .fold(0_i32, |acc, ch| acc.wrapping_mul(31).wrapping_add(ch as i32))
}

#[cfg(test)]
mod tests {
    use crate::core::{authentication::Authentication, authority_utils::AuthorityUtils};

    use super::RememberMeAuthenticationToken;

    #[test]
    fn remember_me_authentication_token_marks_remember_me() {
        let token = RememberMeAuthenticationToken::new(
            "key",
            "alice",
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        assert!(token.is_authenticated());
        assert!(token.is_remember_me());
        assert_eq!(token.get_name(), "alice");
    }
}
