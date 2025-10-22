use crate::core::authz::permission::Permission;

pub trait RolePermissionResolver: Send + Sync {
    fn resolve_permissions_in_role(&self, role_string: &str) -> Vec<Box<dyn Permission>>;
}