use crate::core::authz::permission::role_permission_resolver::RolePermissionResolver;

pub trait RolePermissionResolverAware {
    fn set_role_permission_resolver(&mut self, rpr: impl RolePermissionResolver + 'static);
}
