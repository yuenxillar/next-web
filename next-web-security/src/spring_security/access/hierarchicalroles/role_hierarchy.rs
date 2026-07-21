use std::sync::Arc;

use crate::core::granted_authority::GrantedAuthority;

pub trait RoleHierarchy
where
    Self: Send + Sync,
{
    fn reachable_granted_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>>;
}
