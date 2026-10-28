//! Shiro's in-memory authorization info.

use crate::core::authz::{
    authorization_info::AuthorizationInfo,
    permission::{wildcard_permission::WildcardPermission, Permission},
};

/// The roles and the permissions a realm grants to a principal.
///
/// A role is a name an application checks with `has_role`; a permission is a
/// wildcard string an application checks with `is_permitted`.
#[derive(Clone, Default)]
pub struct SimpleAuthorizationInfo {
    roles: Vec<String>,
    permissions: Vec<String>,
}

impl SimpleAuthorizationInfo {
    /// Creates authorization info with the given roles and permissions.
    ///
    /// # Arguments
    ///
    /// * `roles` - The names of the roles of the principal.
    /// * `permissions` - The wildcard strings of the permissions of the principal.
    pub fn new(
        roles: impl IntoIterator<Item = String>,
        permissions: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            roles: roles.into_iter().collect(),
            permissions: permissions.into_iter().collect(),
        }
    }

    /// Adds a role.
    ///
    /// # Arguments
    ///
    /// * `role` - The name of the role.
    pub fn add_role(&mut self, role: impl ToString) {
        self.roles.push(role.to_string());
    }

    /// Adds a permission.
    ///
    /// # Arguments
    ///
    /// * `permission` - The wildcard string of the permission.
    pub fn add_permission(&mut self, permission: impl ToString) {
        self.permissions.push(permission.to_string());
    }

    /// Returns whether the principal holds the given role.
    ///
    /// # Arguments
    ///
    /// * `role` - The name of the role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|name| name == role)
    }
}

impl AuthorizationInfo for SimpleAuthorizationInfo {
    fn get_roles(&self) -> Vec<String> {
        self.roles.clone()
    }

    fn get_permissions(&self) -> Vec<String> {
        self.permissions.clone()
    }

    fn get_dyn_permissions(&self) -> Vec<Box<dyn Permission>> {
        self.permissions
            .iter()
            .map(|permission| Box::new(WildcardPermission::new(permission)) as Box<dyn Permission>)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::SimpleAuthorizationInfo;
    use crate::core::authz::authorization_info::AuthorizationInfo;

    #[test]
    fn the_roles_and_the_permissions_are_readable() {
        let info = SimpleAuthorizationInfo::new(
            ["admin".to_string()],
            ["printer:*".to_string(), "user:read".to_string()],
        );

        assert_eq!(info.get_roles(), vec!["admin".to_string()]);
        assert_eq!(
            info.get_permissions(),
            vec!["printer:*".to_string(), "user:read".to_string()]
        );
        assert!(info.has_role("admin"));
        assert!(!info.has_role("user"));
    }

    #[test]
    fn the_permissions_are_resolved_to_the_wildcard_grammar() {
        let info = SimpleAuthorizationInfo::new([], ["printer:*".to_string()]);
        let resolved = info.get_dyn_permissions();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].identifier(), "printer:*");
    }

    #[test]
    fn roles_and_permissions_can_be_added_later() {
        let mut info = SimpleAuthorizationInfo::default();
        info.add_role("user");
        info.add_permission("user:read");

        assert!(info.has_role("user"));
        assert_eq!(info.get_permissions(), vec!["user:read".to_string()]);
    }
}
