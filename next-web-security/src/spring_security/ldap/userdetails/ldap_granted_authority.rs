use crate::core::GrantedAuthority;

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

impl GrantedAuthority for LdapGrantedAuthority {
    fn authority(&self) -> Option<&str> {
        Some(self.authority.as_str())
    }
}
