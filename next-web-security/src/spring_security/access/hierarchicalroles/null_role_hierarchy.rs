use std::sync::Arc;

use crate::{
    access::hierarchicalroles::role_hierarchy::RoleHierarchy,
    core::granted_authority::GrantedAuthority,
};

#[derive(Clone, Debug, Default)]
pub struct NullRoleHierarchy;

impl RoleHierarchy for NullRoleHierarchy {
    fn reachable_granted_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        authorities.to_vec()
    }
}
