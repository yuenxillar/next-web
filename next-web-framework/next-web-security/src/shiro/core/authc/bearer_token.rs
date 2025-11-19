use crate::core::{authc::{
    authentication_token::AuthenticationToken, host_authentication_token::HostAuthenticationToken,
}, util::object::Object};

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
        write!(f, "BearerToken [token: {}, host: {}]", self.token, self.host.as_deref().unwrap_or(""))
    }
}