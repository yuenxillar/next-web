use std::{fmt::Debug, ops::Deref, sync::Arc};

use crate::{
    authorization::{AuthorizationDecision, AuthorizationResult},
    core::GrantedAuthority,
};

/// Represents an AuthorizationDecision based on a collection of authorities
#[derive(Clone)]
pub struct AuthorityAuthorizationDecision {
    authorities: Vec<Arc<dyn GrantedAuthority>>,

    base: AuthorizationDecision,
}

impl AuthorityAuthorizationDecision {
    pub fn new(granted: bool, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        Self {
            authorities,
            base: AuthorizationDecision::new(granted),
        }
    }

    pub fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

impl Deref for AuthorityAuthorizationDecision {
    type Target = AuthorizationDecision;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl Debug for AuthorityAuthorizationDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorityAuthorizationDecision")
            .field("granted", &self.is_granted())
            .field(
                "authorities",
                &self
                    .authorities
                    .iter()
                    .map(|a| a.authority().unwrap_or_default())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
