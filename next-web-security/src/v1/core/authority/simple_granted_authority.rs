use std::fmt::Display;

use crate::core::GrantedAuthority;

/// Basic concrete implementation of a GrantedAuthority.
/// Stores a String representation of an authority granted to the Authentication object.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SimpleGrantedAuthority {
    role: String,
}

impl SimpleGrantedAuthority {
    /// Constructs a SimpleGrantedAuthority using the provided authority.
    pub fn new(authority: impl Into<String>) -> Self {
        let authority = authority.into();
        assert!(!authority.trim().is_empty(), "authority cannot be  empty");
        Self { role: authority }
    }
}

impl GrantedAuthority for SimpleGrantedAuthority {
    fn authority(&self) -> Option<&str> {
        Some(self.role.as_str())
    }
}

impl Display for SimpleGrantedAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.role)
    }
}
