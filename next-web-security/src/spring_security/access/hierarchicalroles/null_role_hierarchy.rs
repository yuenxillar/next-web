use crate::access::hierarchicalroles::role_hierarchy::RoleHierarchy;

#[derive(Clone, Debug, Default)]
pub struct NullRoleHierarchy;

impl RoleHierarchy for NullRoleHierarchy {
    fn get_reachable_granted_authorities(&self, authorities: &[String]) -> Vec<String> {
        authorities.to_vec()
    }
}
