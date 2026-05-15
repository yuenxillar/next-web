use crate::core::authentication::Authentication;

pub trait Sid: Send + Sync {
    fn sid_type(&self) -> &'static str;

    fn value(&self) -> &str;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum SidImpl {
    Principal(String),
    GrantedAuthority(String),
}

impl SidImpl {
    pub fn principal(principal: impl Into<String>) -> Self {
        let principal = principal.into();
        assert!(!principal.trim().is_empty(), "Principal required");
        Self::Principal(principal)
    }

    pub fn principal_from_authentication(authentication: &dyn Authentication) -> Self {
        Self::principal(authentication.get_name())
    }

    pub fn granted_authority(authority: impl Into<String>) -> Self {
        let authority = authority.into();
        assert!(!authority.trim().is_empty(), "GrantedAuthority required");
        Self::GrantedAuthority(authority)
    }
}

impl Sid for SidImpl {
    fn sid_type(&self) -> &'static str {
        match self {
            SidImpl::Principal(_) => "PrincipalSid",
            SidImpl::GrantedAuthority(_) => "GrantedAuthoritySid",
        }
    }

    fn value(&self) -> &str {
        match self {
            SidImpl::Principal(value) | SidImpl::GrantedAuthority(value) => value,
        }
    }
}
