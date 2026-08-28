use crate::core::GrantedAuthority;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SimpleGrantedAuthority {
    authority: String,
}

impl SimpleGrantedAuthority {
    pub fn new(authority: impl Into<String>) -> Self {
        let authority = authority.into();
        assert!(
            !authority.trim().is_empty(),
            "authority cannot be null or empty"
        );
        Self { authority }
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }
}

impl GrantedAuthority for SimpleGrantedAuthority {
    fn authority(&self) -> Option<&str> {
        Some(self.authority.as_str())
    }
}
