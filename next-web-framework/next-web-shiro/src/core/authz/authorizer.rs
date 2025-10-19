use crate::core::subject::principal_collection::PrincipalCollection;

use super::authorization_error::AuthorizationError;

pub trait Authorizer
where
    Self: Send + Sync,
{
    // === 权限检查（单个）===
    fn is_permitted(&self, principals: Option<&dyn PrincipalCollection>, permission: &str) -> bool;
    // fn is_permitted(&self, principals: &dyn PrincipalCollection, permission: &dyn Permission) -> bool;

    // === 权限检查（多个）===
    fn is_permitted_all(&self, principals: Option<&dyn PrincipalCollection>, permissions: &[&str]) -> bool;
    // fn is_permitted_all(
    //     &self,
    //     principals: &dyn PrincipalCollection,
    //     permissions: &[Box<dyn Permission>],
    // ) -> bool;

    fn is_permitted_any(
        &self,
        principals: &dyn PrincipalCollection,
        permissions: &[&str],
    ) -> Vec<bool>;
    // fn is_permitted_any(
    //     &self,
    //     principals: &dyn PrincipalCollection,
    //     permissions: &[Box<dyn Permission>],
    // ) -> Vec<bool>;

    // === 权限断言（失败时返回错误）===
    fn check_permission(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError>;

    // fn check_permission(
    //     &self,
    //     principals: &dyn PrincipalCollection,
    //     permission: &dyn Permission,
    // ) -> Result<(), AuthorizationError>;

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError>;

    // fn check_permissions(
    //     &self,
    //     principals: &dyn PrincipalCollection,
    //     permissions: &[Box<dyn Permission>],
    // ) -> Result<(), AuthorizationError>;

    // === 角色检查 ===
    fn has_role(&self, principals: Option<&dyn PrincipalCollection>, role: &str) -> bool;
    fn has_all_roles(&self, principals: Option<&dyn PrincipalCollection>, roles: &[&str]) -> bool;
    fn has_any_roles(&self, principals: Option<&dyn PrincipalCollection>, roles: &[&str]) -> Vec<bool>;

    // === 角色断言 ===
    fn check_role(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError>;
    fn check_roles(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError>;
}
