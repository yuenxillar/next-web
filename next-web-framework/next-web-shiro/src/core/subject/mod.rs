pub mod subject_context;
pub mod principal_collection;
pub mod support;

use std::{any::Any, sync::Arc};

use principal_collection::PrincipalCollection;

use super::{
    authc::{authentication_error::AuthenticationError, authentication_token::AuthenticationToken},
    authz::authorization_error::AuthorizationError,
    object::Object,
    session::Session,
};

pub trait Subject
where
    Self: Send + Sync,
    Self: Any
{
    // === 身份相关 ===
    fn get_principal(&self) -> Option<&Object>;
    fn get_principals(&self) -> Option<Arc<dyn PrincipalCollection>>;

    // === 认证状态 ===
    fn is_authenticated(&self) -> bool;
    fn is_remembered(&self) -> bool;

    // === 授权（权限）===
    fn is_permitted(&self, permission: &str) -> bool;
    fn is_permitted_all(&self, permissions: &[&str]) -> bool;
    fn check_permission(&self, permission: &str) -> Result<(), AuthorizationError>;
    fn check_permissions(&self, permissions: &[&str]) -> Result<(), AuthorizationError>;

    // === 授权（角色）===
    fn has_role(&self, role: &str) -> bool;
    fn has_all_roles(&self, roles: &[&str]) -> bool;
    fn check_role(&self, role: &str) -> Result<(), AuthorizationError>;
    fn check_roles(&self, roles: &[&str]) -> Result<(), AuthorizationError>;

    // === 会话 ===
    fn get_session(&self) -> Option<&dyn Session>;
    fn get_session_or_create(&self, create: bool) -> Option<Arc<dyn Session>>;

    // === 登录/登出 ===
    fn login(&mut self, token: &dyn AuthenticationToken) -> Result<(), AuthenticationError>;
    fn logout(&mut self);

    fn run_as(&mut self, principals: Vec<Object>) -> Result<(), String>;
    fn is_run_as(&self) -> bool;
    fn get_previous_principals(&self) -> Option<Vec<Object>>;
    fn release_run_as(&mut self) -> Option<Vec<Object>>;
}
