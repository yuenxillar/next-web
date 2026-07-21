use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::core::{granted_authority::GrantedAuthority, Authentication};

#[derive(Clone, Default)]
pub struct PreAuthenticatedAuthenticationToken {
    principal: Option<String>,
    credentials: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AnyValue>,
    authenticated: bool,
}

impl PreAuthenticatedAuthenticationToken {
    pub fn unauthenticated(principal: Option<String>, credentials: Option<String>) -> Self {
        Self {
            principal,
            credentials,
            authorities: Vec::new(),
            details: None,
            authenticated: false,
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
            authorities,
            details: None,
            authenticated: true,
        }
    }

    pub fn set_details(&mut self, details: Option<String>) {
        self.details = details.map(AnyValue::from);
    }

    pub fn set_details_value(&mut self, details: Option<AnyValue>) {
        self.details = details;
    }
}

// impl Authentication for PreAuthenticatedAuthenticationToken {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }

//     fn authentication_type(&self) -> &'static str {
//         std::any::type_name::<Self>()
//     }

//     fn get_credentials(&self) -> Option<String> {
//         self.credentials.clone()
//     }

//     fn get_details_ref(&self) -> Option<&AnyValue> {
//         self.details.as_ref()
//     }

//     fn get_principal(&self) -> Option<String> {
//         self.principal.clone()
//     }

//     fn is_authenticated(&self) -> bool {
//         self.authenticated
//     }

//     fn authorities(&self) -> Vec<String> {
//         self.authorities
//             .iter()
//             .filter_map(|authority| authority.authority().map(ToString::to_string))
//             .collect()
//     }
// }

#[cfg(test)]
mod tests {
    use crate::core::{authority_utils::AuthorityUtils, Authentication};

    use super::PreAuthenticatedAuthenticationToken;

    #[test]
    fn pre_authenticated_authentication_token_supports_authenticated_constructor() {
        let token = PreAuthenticatedAuthenticationToken::authenticated(
            "alice",
            Some(String::from("external-credential")),
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        assert!(token.is_authenticated());
        assert_eq!(token.get_name(), "alice");
    }
}
