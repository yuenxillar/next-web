use crate::core::authz::permission::{
    permission_resolver::PermissionResolver, wildcard_permission::WildcardPermission, Permission,
};

/// Resolves the wildcard grammar of Shiro: `printer:print`, `printer:*`.
#[derive(Clone, Default)]
pub struct WildcardPermissionResolver {
    case_sensitive: bool,
}

impl WildcardPermissionResolver {
    /// Creates a resolver that matches the parts case insensitively.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a resolver whose permissions keep their case.
    ///
    /// # Arguments
    ///
    /// * `case_sensitive` - Whether the parts of a permission keep their case.
    pub fn with_case_sensitive(case_sensitive: bool) -> Self {
        Self { case_sensitive }
    }

    /// Returns whether the permissions this resolver creates keep their case.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

impl PermissionResolver for WildcardPermissionResolver {
    fn resolve_permission(&self, permission: &str) -> Box<dyn Permission> {
        Box::new(WildcardPermission::with_case_sensitive(
            permission,
            self.case_sensitive,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::WildcardPermissionResolver;
    use crate::core::authz::permission::permission_resolver::PermissionResolver;

    #[test]
    fn a_resolved_permission_keeps_the_wildcard_grammar() {
        let resolver = WildcardPermissionResolver::new();
        let granted = resolver.resolve_permission("printer:*");
        let requested = resolver.resolve_permission("printer:print");

        assert!(granted.implies(requested.as_ref()));
        assert_eq!(granted.identifier(), "printer:*");
    }

    #[test]
    fn a_case_sensitive_resolver_does_not_lowercase_the_parts() {
        let resolver = WildcardPermissionResolver::with_case_sensitive(true);

        assert_eq!(
            resolver.resolve_permission("Printer:Print").identifier(),
            "Printer:Print"
        );
        assert!(resolver.is_case_sensitive());
    }
}
