use std::{ops::Deref, sync::Arc};

use crate::{authorization::AuthorizationDecision, core::GrantedAuthority};

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
