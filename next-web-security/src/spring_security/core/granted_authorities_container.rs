use std::sync::Arc;

use crate::core::granted_authority::GrantedAuthority;

pub trait GrantedAuthoritiesContainer: Send + Sync {
    fn granted_authorities(&self) -> Vec<Arc<dyn GrantedAuthority>>;
}
