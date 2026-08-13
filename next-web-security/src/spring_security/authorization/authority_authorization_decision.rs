use std::sync::Arc;

use crate::{authorization::AuthorizationResult, core::GrantedAuthority};

#[derive(Clone)]
pub struct AuthorityAuthorizationDecision {
    granted: bool,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl AuthorityAuthorizationDecision {
    pub fn new(granted: bool, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        Self {
            granted,
            authorities,
        }
    }

    pub fn is_granted(&self) -> bool {
        self.granted
    }

    pub fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

impl AuthorizationResult for AuthorityAuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.is_granted()
    }
}
