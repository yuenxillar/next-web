use next_web_core::async_trait;

use crate::core::granted_authority::GrantedAuthority;

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

#[async_trait]
impl GrantedAuthority for SimpleGrantedAuthority {
    async fn get_authority(&self) -> Option<String> {
        Some(self.authority.clone())
    }
}
