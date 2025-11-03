use crate::core::authz::permission::permission_resolver::PermissionResolver;


pub struct WildcardPermissionResolver {}


impl PermissionResolver  for  WildcardPermissionResolver {
    fn resolve_permission(&self, permission: &str) -> Box<dyn super::Permission> {
        todo!()
    }
}

impl Default for WildcardPermissionResolver {
    fn default() -> Self {
        Self {  }
    }
}