use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::core::{granted_authority::GrantedAuthority, Authentication};

#[derive(Clone, Default)]
pub struct SimpleAuthentication {
    principal: Option<String>,
    credentials: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AnyValue>,
    authenticated: bool,
}

impl SimpleAuthentication {
    pub fn builder() -> SimpleAuthenticationBuilder {
        SimpleAuthenticationBuilder::default()
    }

    pub fn builder_from(authentication: &dyn Authentication) -> SimpleAuthenticationBuilder {
        SimpleAuthenticationBuilder {
            principal: authentication.get_principal(),
            credentials: authentication.get_credentials(),
            authorities: crate::core::authority_utils::AuthorityUtils::create_authority_list(
                authentication.authorities(),
            ),
            details: authentication.get_details_value(),
            authenticated: authentication.is_authenticated(),
        }
    }
}

// impl Authentication for SimpleAuthentication {
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

//     fn set_authenticated(&mut self, _is_authenticated: bool) -> Result<(), &'static str> {
//         Err("Instead of calling this setter, please call builder to create a new instance")
//     }

//     fn authorities(&self) -> Vec<String> {
//         self.authorities
//             .iter()
//             .filter_map(|authority| authority.authority())
//             .map(ToString::to_string)
//             .collect()
//     }
// }

#[derive(Clone, Default)]
pub struct SimpleAuthenticationBuilder {
    principal: Option<String>,
    credentials: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AnyValue>,
    authenticated: bool,
}

impl SimpleAuthenticationBuilder {
    pub fn principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    pub fn credentials(mut self, credentials: impl Into<String>) -> Self {
        self.credentials = Some(credentials.into());
        self
    }

    pub fn authorities(mut self, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        self.authorities = authorities;
        self
    }

    pub fn details(mut self, details: AnyValue) -> Self {
        self.details = Some(details);
        self
    }

    pub fn authenticated(mut self, authenticated: bool) -> Self {
        self.authenticated = authenticated;
        self
    }

    pub fn build(self) -> SimpleAuthentication {
        SimpleAuthentication {
            principal: self.principal,
            credentials: self.credentials,
            authorities: self.authorities,
            details: self.details,
            authenticated: self.authenticated,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{
        authority_utils::AuthorityUtils, simple_authentication::SimpleAuthentication,
        Authentication,
    };

    #[test]
    fn simple_authentication_builder_creates_authenticated_instance() {
        let authentication = SimpleAuthentication::builder()
            .principal("alice")
            .credentials("secret")
            .authorities(AuthorityUtils::create_authority_list(["ROLE_USER"]))
            .authenticated(true)
            .build();

        assert_eq!(authentication.get_name(), "alice");
        assert_eq!(
            authentication.get_credentials(),
            Some(String::from("secret"))
        );
        assert_eq!(
            authentication.authorities(),
            vec![String::from("ROLE_USER")]
        );
        assert!(authentication.is_authenticated());
    }

    #[test]
    fn builder_from_copies_existing_authentication() {
        let original = SimpleAuthentication::builder()
            .principal("alice")
            .authorities(AuthorityUtils::create_authority_list(["ROLE_USER"]))
            .authenticated(true)
            .build();

        let copied = SimpleAuthentication::builder_from(&original)
            .authenticated(false)
            .build();

        assert_eq!(copied.get_name(), "alice");
        assert_eq!(copied.authorities(), vec![String::from("ROLE_USER")]);
        assert!(!copied.is_authenticated());
    }
}
