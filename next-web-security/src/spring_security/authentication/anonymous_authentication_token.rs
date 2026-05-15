use std::sync::Arc;

use crate::core::{
    authentication::Authentication, granted_authority::GrantedAuthority,
};

#[derive(Clone, Default)]
pub struct AnonymousAuthenticationToken {
    principal: String,
    key_hash: i32,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl AnonymousAuthenticationToken {
    pub fn new(
        key: impl AsRef<str>,
        principal: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let principal = principal.into();
        assert!(!key.as_ref().trim().is_empty(), "key cannot be null or empty");
        assert!(
            !principal.trim().is_empty(),
            "principal cannot be null or empty"
        );
        assert!(
            !authorities.is_empty(),
            "authorities cannot be null or empty"
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

impl Authentication for AnonymousAuthenticationToken {
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

    fn is_anonymous(&self) -> bool {
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

    use super::AnonymousAuthenticationToken;

    #[test]
    fn anonymous_authentication_token_is_authenticated_and_anonymous() {
        let token = AnonymousAuthenticationToken::new(
            "key",
            "anonymousUser",
            AuthorityUtils::create_authority_list(["ROLE_ANONYMOUS"]),
        );

        assert!(token.is_authenticated());
        assert!(token.is_anonymous());
        assert_eq!(token.get_name(), "anonymousUser");
        assert_eq!(token.get_credentials(), Some(String::new()));
    }
}
