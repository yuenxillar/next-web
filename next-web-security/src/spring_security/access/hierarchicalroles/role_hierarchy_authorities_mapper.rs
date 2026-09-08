use std::sync::Arc;

use crate::{
    access::hierarchicalroles::role_hierarchy::RoleHierarchy,
    core::{authority::mapping::GrantedAuthoritiesMapper, GrantedAuthority},
};

pub struct RoleHierarchyAuthoritiesMapper {
    role_hierarchy: Arc<dyn RoleHierarchy>,
}

impl RoleHierarchyAuthoritiesMapper {
    pub fn new(role_hierarchy: Arc<dyn RoleHierarchy>) -> Self {
        Self { role_hierarchy }
    }
}

impl GrantedAuthoritiesMapper for RoleHierarchyAuthoritiesMapper {
    fn map_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        // let names = authorities
        //     .into_iter()
        //     .filter_map(|authority| authority.authority().map(ToString::to_string))
        //     .collect::<Vec<_>>();

        // self.role_hierarchy
        //     .reachable_granted_authorities(&names)
        //     .into_iter()
        //     .map(|authority| {
        //         Arc::new(SimpleGrantedAuthority::new(authority)) as Arc<dyn GrantedAuthority>
        //     })
        //     .collect()
        todo!()
    }
}
