use std::sync::Arc;

use std::collections::HashSet;

use crate::{
    access::hierarchicalroles::{
        null_role_hierarchy::NullRoleHierarchy, role_hierarchy::RoleHierarchy,
    },
    core::Authentication,
};

pub struct AuthoritiesAuthorizationManager {
    role_hierarchy: Arc<dyn RoleHierarchy>,
}

impl AuthoritiesAuthorizationManager {
    pub fn new() -> Self {
        Self {
            role_hierarchy: Arc::new(NullRoleHierarchy),
        }
    }

    pub fn set_role_hierarchy(&mut self, role_hierarchy: Arc<dyn RoleHierarchy>) {
        self.role_hierarchy = role_hierarchy;
    }

    pub fn is_authorized(
        &self,
        authentication: &dyn Authentication,
        required_authorities: &HashSet<String>,
    ) -> bool {
        if required_authorities.is_empty() {
            return false;
        }

        self.role_hierarchy
            .get_reachable_granted_authorities(&authentication.authorities())
            .into_iter()
            .any(|authority| required_authorities.contains(&authority))
    }
}

impl Default for AuthoritiesAuthorizationManager {
    fn default() -> Self {
        Self::new()
    }
}
