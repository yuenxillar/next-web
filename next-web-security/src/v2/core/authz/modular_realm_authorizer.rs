use std::sync::Arc;

use crate::core::{
    authz::{
        authorization_error::AuthorizationError, authorizer::Authorizer, permission::Permission,
    },
    realm::Realm,
    subject::principal_collection::PrincipalCollection,
};

/// An authorizer that asks every authorizing realm of the application.
///
/// A permission is granted when any realm grants it, and a role is held when any
/// realm holds it; a check that needs every permission or every role calls the
/// single checks in turn. A configuration with no realm refuses everything, the
/// way Shiro does.
#[derive(Clone, Default)]
pub struct ModularRealmAuthorizer {
    authorizing_realms: Vec<Arc<dyn Authorizer>>,
}

impl ModularRealmAuthorizer {
    /// Creates an authorizer over the given authorizing realms.
    ///
    /// # Arguments
    ///
    /// * `authorizing_realms` - The realms that authorize, in the order they are
    ///   asked.
    pub fn new(authorizing_realms: Vec<Arc<dyn Authorizer>>) -> Self {
        Self { authorizing_realms }
    }

    /// Replaces the authorizing realms of this authorizer.
    ///
    /// # Arguments
    ///
    /// * `authorizing_realms` - The realms that authorize, in the order they are
    ///   asked.
    pub fn set_authorizing_realms(&mut self, authorizing_realms: Vec<Arc<dyn Authorizer>>) {
        self.authorizing_realms = authorizing_realms;
    }

    /// Replaces the authorizing realms with the realms that authorize.
    ///
    /// A realm that also authorizes contributes its own checks; a realm that only
    /// authenticates contributes none.
    ///
    /// # Arguments
    ///
    /// * `realms` - The realms of the application.
    pub fn set_realms(&mut self, realms: Vec<Arc<dyn Realm>>) {
        self.authorizing_realms = realms
            .iter()
            .filter_map(|realm| realm.as_authorizer())
            .collect();
    }

    /// Adds an authorizing realm.
    ///
    /// # Arguments
    ///
    /// * `authorizing_realm` - The realm to ask after the ones already held.
    pub fn add_authorizing_realm(&mut self, authorizing_realm: Arc<dyn Authorizer>) {
        self.authorizing_realms.push(authorizing_realm);
    }

    /// Returns the authorizing realms of this authorizer.
    pub fn authorizing_realms(&self) -> &[Arc<dyn Authorizer>] {
        &self.authorizing_realms
    }

    /// Returns whether this authorizer holds no realm at all.
    pub fn is_empty(&self) -> bool {
        self.authorizing_realms.is_empty()
    }
}

impl Authorizer for ModularRealmAuthorizer {
    fn is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        self.authorizing_realms
            .iter()
            .any(|authorizer| authorizer.is_permitted(principal, permission))
    }

    fn is_permitted_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        self.authorizing_realms
            .iter()
            .any(|authorizer| authorizer.is_permitted_from_str(principal, permission))
    }

    fn is_permitted_from_str_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        permissions
            .iter()
            .map(|permission| self.is_permitted_from_str(principal, permission))
            .collect()
    }

    fn is_permitted_from_permission_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        permissions
            .iter()
            .map(|permission| self.is_permitted(principal, permission.as_ref()))
            .collect()
    }

    fn is_permitted_all(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        permissions
            .iter()
            .all(|permission| self.is_permitted(principal, permission.as_ref()))
    }

    fn is_permitted_all_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        permissions
            .iter()
            .all(|permission| self.is_permitted_from_str(principal, permission))
    }

    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        if self.is_permitted(principal, permission) {
            Ok(())
        } else {
            Err(AuthorizationError::forbidden(permission.identifier()))
        }
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        if self.is_permitted_from_str(principal, permission) {
            Ok(())
        } else {
            Err(AuthorizationError::forbidden(permission))
        }
    }

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        for permission in permissions {
            self.check_permission(principals, permission.as_ref())?;
        }

        Ok(())
    }

    fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        for permission in permissions {
            self.check_permission_from_str(principals, permission)?;
        }

        Ok(())
    }

    fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool {
        self.authorizing_realms
            .iter()
            .any(|authorizer| authorizer.has_role(principal, role_identifier))
    }

    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        role_identifiers
            .iter()
            .map(|role| self.has_role(principal, role))
            .collect()
    }

    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        role_identifiers
            .iter()
            .all(|role| self.has_role(principal, role))
    }

    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        if self.has_role(principal, role) {
            Ok(())
        } else {
            Err(AuthorizationError::forbidden(role))
        }
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        for role in roles {
            self.check_role(principal, role)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::Id;

    use super::ModularRealmAuthorizer;
    use crate::core::{
        authz::{
            authorization_error::AuthorizationError,
            authorization_info::AuthorizationInfo,
            authorizer::Authorizer,
            permission::{
                permission_resolver::PermissionResolver, Permission, WildcardPermissionResolver,
            },
            simple_authorization_info::SimpleAuthorizationInfo,
        },
        subject::{
            principal_collection::PrincipalCollection,
            simple_principal_collection::SimplePrincipalCollection,
        },
        util::object::Object,
    };

    /// An authorizing realm that grants a fixed set of roles and permissions.
    struct StubAuthorizer {
        principals: SimplePrincipalCollection,
        info: SimpleAuthorizationInfo,
    }

    impl Authorizer for StubAuthorizer {
        fn is_permitted(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permission: &dyn Permission,
        ) -> bool {
            if !Self::mine(&self.principals, principal) {
                return false;
            }

            self.info
                .get_dyn_permissions()
                .iter()
                .any(|granted| granted.implies(permission))
        }

        fn is_permitted_from_str(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permission: &str,
        ) -> bool {
            self.is_permitted(
                principal,
                WildcardPermissionResolver::default()
                    .resolve_permission(permission)
                    .as_ref(),
            )
        }

        fn is_permitted_from_str_list(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permissions: &[&str],
        ) -> Vec<bool> {
            permissions
                .iter()
                .map(|permission| self.is_permitted_from_str(principal, permission))
                .collect()
        }

        fn is_permitted_from_permission_list(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permissions: &[Box<dyn Permission>],
        ) -> Vec<bool> {
            permissions
                .iter()
                .map(|permission| self.is_permitted(principal, permission.as_ref()))
                .collect()
        }

        fn is_permitted_all(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permissions: &[Box<dyn Permission>],
        ) -> bool {
            permissions
                .iter()
                .all(|permission| self.is_permitted(principal, permission.as_ref()))
        }

        fn is_permitted_all_from_str(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permissions: &[&str],
        ) -> bool {
            permissions
                .iter()
                .all(|permission| self.is_permitted_from_str(principal, permission))
        }

        fn check_permission(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permission: &dyn Permission,
        ) -> Result<(), AuthorizationError> {
            if self.is_permitted(principal, permission) {
                Ok(())
            } else {
                Err(AuthorizationError::forbidden(permission.identifier()))
            }
        }

        fn check_permission_from_str(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            permission: &str,
        ) -> Result<(), AuthorizationError> {
            if self.is_permitted_from_str(principal, permission) {
                Ok(())
            } else {
                Err(AuthorizationError::forbidden(permission))
            }
        }

        fn check_permissions(
            &self,
            principals: Option<&dyn PrincipalCollection>,
            permissions: &[Box<dyn Permission>],
        ) -> Result<(), AuthorizationError> {
            for permission in permissions {
                self.check_permission(principals, permission.as_ref())?;
            }

            Ok(())
        }

        fn check_permissions_from_str(
            &self,
            principals: Option<&dyn PrincipalCollection>,
            permissions: &[&str],
        ) -> Result<(), AuthorizationError> {
            for permission in permissions {
                self.check_permission_from_str(principals, permission)?;
            }

            Ok(())
        }

        fn has_role(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            role_identifier: &str,
        ) -> bool {
            Self::mine(&self.principals, principal) && self.info.has_role(role_identifier)
        }

        fn has_roles(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            role_identifiers: &[&str],
        ) -> Vec<bool> {
            role_identifiers
                .iter()
                .map(|role| self.has_role(principal, role))
                .collect()
        }

        fn has_all_roles(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            role_identifiers: &[&str],
        ) -> bool {
            role_identifiers
                .iter()
                .all(|role| self.has_role(principal, role))
        }

        fn check_role(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            role: &str,
        ) -> Result<(), AuthorizationError> {
            if self.has_role(principal, role) {
                Ok(())
            } else {
                Err(AuthorizationError::forbidden(role))
            }
        }

        fn check_roles(
            &self,
            principal: Option<&dyn PrincipalCollection>,
            roles: &[&str],
        ) -> Result<(), AuthorizationError> {
            for role in roles {
                self.check_role(principal, role)?;
            }

            Ok(())
        }
    }

    impl StubAuthorizer {
        /// Returns whether the principals of the request are the ones of the realm.
        fn mine(
            principals: &SimplePrincipalCollection,
            principal: Option<&dyn PrincipalCollection>,
        ) -> bool {
            match principal {
                Some(principal) => principal.id() == principals.id(),
                None => false,
            }
        }
    }

    fn principals(name: &str) -> Arc<dyn PrincipalCollection> {
        Arc::new(SimplePrincipalCollection::new(
            [Object::Str(name.to_string())],
            "stubRealm",
        ))
    }

    fn authorizer(name: &str, roles: &[&str], permissions: &[&str]) -> StubAuthorizer {
        StubAuthorizer {
            principals: SimplePrincipalCollection::new(
                [Object::Str(name.to_string())],
                "stubRealm",
            ),
            info: SimpleAuthorizationInfo::new(
                roles.iter().map(ToString::to_string),
                permissions.iter().map(ToString::to_string),
            ),
        }
    }

    #[test]
    fn any_realm_can_grant_a_permission() {
        let authorizer = ModularRealmAuthorizer::new(vec![
            Arc::new(authorizer("root", &[], &["printer:print"])) as Arc<dyn Authorizer>,
            Arc::new(authorizer("root", &[], &["user:read"])) as Arc<dyn Authorizer>,
        ]);
        let principals = principals("root");

        assert!(authorizer.is_permitted_from_str(Some(principals.as_ref()), "printer:print"));
        assert!(authorizer.is_permitted_from_str(Some(principals.as_ref()), "user:read"));
        assert!(!authorizer.is_permitted_from_str(Some(principals.as_ref()), "printer:query"));
    }

    #[test]
    fn a_permission_and_a_role_are_needed_for_the_all_checks() {
        let authorizer = ModularRealmAuthorizer::new(vec![
            Arc::new(authorizer("root", &["admin"], &["printer:*"])) as Arc<dyn Authorizer>,
            Arc::new(authorizer("root", &["user"], &["user:read"])) as Arc<dyn Authorizer>,
        ]);
        let principals = principals("root");

        assert!(authorizer.is_permitted_all_from_str(
            Some(principals.as_ref()),
            &["printer:print", "user:read"]
        ));
        assert!(!authorizer.is_permitted_all_from_str(
            Some(principals.as_ref()),
            &["printer:print", "user:write"]
        ));
        assert!(authorizer.has_all_roles(Some(principals.as_ref()), &["admin", "user"]));
        assert!(!authorizer.has_all_roles(Some(principals.as_ref()), &["admin", "auditor"]));
    }

    #[test]
    fn a_refused_check_reports_the_right_it_asked_for() {
        let authorizer = ModularRealmAuthorizer::new(vec![
            Arc::new(authorizer("root", &["admin"], &["printer:*"])) as Arc<dyn Authorizer>,
        ]);
        let principals = principals("root");

        let error = authorizer
            .check_permission_from_str(Some(principals.as_ref()), "user:read")
            .expect_err("the permission is not granted");
        assert_eq!(error.message(), "user:read");

        assert!(authorizer
            .check_role(Some(principals.as_ref()), "admin")
            .is_ok());
        assert!(authorizer
            .check_roles(Some(principals.as_ref()), &["admin", "auditor"])
            .is_err());
    }

    #[test]
    fn a_realm_ignores_the_principals_it_did_not_authenticate() {
        let authorizer = ModularRealmAuthorizer::new(vec![
            Arc::new(authorizer("root", &["admin"], &["printer:*"])) as Arc<dyn Authorizer>,
        ]);

        assert!(!authorizer.is_permitted_from_str(Some(principals("other").as_ref()), "printer:*"));
        assert!(!authorizer.is_permitted_from_str(None, "printer:*"));
    }

    #[test]
    fn an_authorizer_without_a_realm_refuses_everything() {
        let authorizer = ModularRealmAuthorizer::default();
        let principals = principals("root");

        assert!(authorizer.is_empty());
        assert!(!authorizer.is_permitted_from_str(Some(principals.as_ref()), "printer:*"));
        assert!(authorizer
            .check_permission_from_str(Some(principals.as_ref()), "printer:*")
            .is_err());
        assert!(authorizer
            .check_roles(Some(principals.as_ref()), &["admin"])
            .is_err());
    }

    #[test]
    fn the_roles_and_the_permissions_of_one_realm_are_readable() {
        let authorizer = ModularRealmAuthorizer::new(vec![
            Arc::new(authorizer("root", &["admin", "user"], &["printer:print"]))
                as Arc<dyn Authorizer>,
        ]);
        let principals = principals("root");

        assert_eq!(
            authorizer.has_roles(Some(principals.as_ref()), &["admin", "auditor"]),
            vec![true, false]
        );
        assert_eq!(
            authorizer.is_permitted_from_str_list(
                Some(principals.as_ref()),
                &["printer:print", "printer:query"]
            ),
            vec![true, false]
        );
    }
}
