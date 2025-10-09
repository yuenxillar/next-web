use std::sync::Arc;

use crate::access::hierarchicalroles::role_hierarchy::RoleHierarchy;

pub struct AuthoritiesAuthorizationManager {
    role_hierarchy: Arc<dyn RoleHierarchy>,
}

impl AuthoritiesAuthorizationManager {
    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }
}
