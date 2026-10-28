use crate::{
    authentication::BaseAuthenticationToken,
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};
use next_web_core::error::BoxError;
use std::{any::TypeId, borrow::Cow, fmt::Debug, sync::Arc};

#[derive(Clone, Debug, Default)]
pub struct LdapAuthenticationRequest {
    username: String,
    password: String,
    authenticated: bool,
    authorities: Vec<String>,
    base: BaseAuthenticationToken,
}

impl LdapAuthenticationRequest {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            authenticated: false,
            authorities: Vec::new(),
            base: BaseAuthenticationToken::new(None),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn set_authenticated(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
    }

    pub fn set_authorities(&mut self, authorities: Vec<String>) {
        self.authorities = authorities;
        self.base = BaseAuthenticationToken::new(Some(
            self.authorities
                .iter()
                .map(|v| {
                    Arc::new(crate::core::authority::SimpleGrantedAuthority::new(v))
                        as Arc<dyn GrantedAuthority>
                })
                .collect(),
        ));
        let _ = self.base.set_authenticated(self.authenticated);
    }
}

impl Authentication for LdapAuthenticationRequest {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }
    fn credentials(&self) -> Option<&AuthPrincipal> {
        None
    }
    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }
    fn principal(&self) -> Option<&AuthPrincipal> {
        None
    }
    fn is_authenticated(&self) -> bool {
        self.authenticated
    }
    fn set_authenticated(&mut self, v: bool) -> Result<(), BoxError> {
        self.authenticated = v;
        self.base.set_authenticated(v)
    }
    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(crate::core::SimpleAuthenticationBuilder::new(self))
    }
    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
impl Principal for LdapAuthenticationRequest {
    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.username)
    }
}

// impl Authentication for LdapAuthenticationRequest {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }

//     fn authentication_type(&self) -> &'static str {
//         std::any::type_name::<Self>()
//     }

//     fn is_authenticated(&self) -> bool {
//         self.authenticated
//     }

//     fn authorities(&self) -> Vec<String> {
//         self.authorities.clone()
//     }
// }
