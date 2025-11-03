use crate::core::authz::permission::Permission;


pub trait PermissionResolver: Send + Sync {
    
    fn resolve_permission(&self, permission: &str) -> Box<dyn Permission>;
}