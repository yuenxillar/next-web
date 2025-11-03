use crate::core::{authz::permission::Permission, subject::principal_collection::PrincipalCollection};

use super::authorization_error::AuthorizationError;


pub trait Authorizer
where
    Self: Send + Sync,
{
    // === 权限检查（单个）===
     fn is_permitted(&self, principal: Option<&dyn PrincipalCollection>, permission: &dyn Permission) -> bool;
     fn is_permitted_from_str(&self, principal: Option<&dyn PrincipalCollection>, permission: &str) -> bool;
     fn is_permitted_from_str_list(&self, principal: Option<&dyn PrincipalCollection>, permissions: &[&str]) -> Vec<bool>;
     fn is_permitted_from_permission_list(&self, principal: Option<&dyn PrincipalCollection>, permissions: &[Box<dyn Permission>]) -> Vec<bool>;

    // === 权限检查（多个）===
     fn is_permitted_all(&self, principal: Option<&dyn PrincipalCollection>, permissions: &[Box<dyn Permission>]) -> bool;
     fn is_permitted_all_from_str(&self, principal: Option<&dyn PrincipalCollection>, permissions: &[&str]) -> bool;
    
    // === 权限断言（失败时返回错误）===
     fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError>;

     fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError>;

     fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError>;

     fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError>;

    // === 角色检查 ===
     fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool;
     fn has_roles(&self, principal: Option<&dyn PrincipalCollection>, role_identifiers: &[&str]) -> Vec<bool>;
     fn has_all_roles(&self, principal: Option<&dyn PrincipalCollection>, role_identifiers: &[&str]) -> bool;

    // === 角色断言 ===
     fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError>;

     fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError>;

}
