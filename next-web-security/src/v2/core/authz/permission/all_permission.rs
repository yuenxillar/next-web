//! Shiro's `AllPermission`: the right to do everything.

use std::fmt::Display;

use super::Permission;

/// A permission that implies every other permission.
///
/// It is what a wildcard of the whole string (`*`) is in the wildcard grammar,
/// and it is the permission an administrator account is given when the
/// application does not want to enumerate the rights one by one.
///
/// ```
/// use next_web_security::core::authz::permission::{
///     all_permission::AllPermission, wildcard_permission::WildcardPermission, Permission,
/// };
///
/// let all = AllPermission;
/// assert!(all.implies(&WildcardPermission::new("printer:print")));
/// assert!(all.implies(&AllPermission));
/// assert!(!WildcardPermission::new("printer:print").implies(&all));
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AllPermission;

impl AllPermission {
    /// The identifier of the permission.
    pub const IDENTIFIER: &'static str = "*";
}

impl Permission for AllPermission {
    fn implies(&self, _p: &dyn Permission) -> bool {
        true
    }

    fn identifier(&self) -> &str {
        Self::IDENTIFIER
    }
}

impl Display for AllPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::IDENTIFIER)
    }
}

#[cfg(test)]
mod tests {
    use super::AllPermission;
    use crate::core::authz::permission::{
        permission_resolver::PermissionResolver, wildcard_permission::WildcardPermission,
        wildcard_permission_resolver::WildcardPermissionResolver, Permission,
    };

    #[test]
    fn everything_is_implied_by_it() {
        assert!(AllPermission.implies(&WildcardPermission::new("printer:print")));
        assert!(AllPermission.implies(&WildcardPermission::new("user:delete,read")));
        assert!(AllPermission.implies(&AllPermission));
    }

    #[test]
    fn a_wildcard_permission_does_not_imply_it() {
        let printer = WildcardPermission::new("printer:print");

        assert!(!printer.implies(&AllPermission));
        assert_eq!(AllPermission.identifier(), "*");
        assert_eq!(AllPermission.to_string(), "*");
    }

    #[test]
    fn a_resolved_wildcard_of_everything_implies_a_permission() {
        let granted = WildcardPermissionResolver::default().resolve_permission(AllPermission::IDENTIFIER);

        assert!(granted.implies(&WildcardPermission::new("printer:print")));
    }
}
