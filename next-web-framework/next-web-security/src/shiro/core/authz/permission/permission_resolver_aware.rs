use crate::core::authz::permission::permission_resolver::PermissionResolver;

pub trait PermissionResolverAware {
    fn set_permission_resolver(&mut self, resolver: impl  PermissionResolver + 'static);
}