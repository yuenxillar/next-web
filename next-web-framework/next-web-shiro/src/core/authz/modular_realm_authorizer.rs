use crate::core::{authz::{authorization_error::AuthorizationError, authorizer::Authorizer, permission::Permission}, realm::Realm, subject::principal_collection::PrincipalCollection};

#[derive(Clone)]
pub struct ModularRealmAuthorizer {}

impl ModularRealmAuthorizer {
    pub fn set_realms<T: Realm>(&mut self, realms: Vec<T>) {}
}

impl Authorizer for ModularRealmAuthorizer {
    fn is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        todo!()
    }

    fn is_permitted_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        todo!()
    }

    fn is_permitted_from_str_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        todo!()
    }

    fn is_permitted_from_permission_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        todo!()
    }

    fn is_permitted_all(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        todo!()
    }

    fn is_permitted_all_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        todo!()
    }

    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        todo!()
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        todo!()
    }

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        todo!()
    }

    fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        todo!()
    }

    fn has_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifier: &str,
    ) -> bool {
        todo!()
    }

    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        todo!()
    }

    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        todo!()
    }

    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        todo!()
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        todo!()
    }
}
impl Default for ModularRealmAuthorizer {
    fn default() -> Self {
        Self {}
    }
}
