use next_web_core::async_trait;

use crate::core::granted_authority::GrantedAuthority;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct LdapGrantedAuthority {
    authority: String,
}

impl LdapGrantedAuthority {
    pub fn new(authority: impl Into<String>) -> Self {
        Self {
            authority: authority.into(),
        }
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }
}

#[async_trait]
impl GrantedAuthority for LdapGrantedAuthority {
    async fn get_authority(&self) -> Option<String> {
        Some(self.authority.clone())
    }
}
