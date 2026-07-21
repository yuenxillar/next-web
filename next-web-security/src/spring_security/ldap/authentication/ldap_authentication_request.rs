use crate::core::Authentication;

#[derive(Clone, Debug, Default)]
pub struct LdapAuthenticationRequest {
    username: String,
    password: String,
    authenticated: bool,
    authorities: Vec<String>,
}

impl LdapAuthenticationRequest {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            authenticated: false,
            authorities: Vec::new(),
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
