pub mod principal_collection;
pub mod subject_context;
pub mod support;

use std::{any::Any, sync::Arc};

#[cfg(feature = "web")]
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};
use next_web_core::{
    async_trait,
    error::{illegal_state_error::IllegalStateError, BoxError},
    DynClone,
};
use principal_collection::PrincipalCollection;

#[cfg(feature = "web")]
use crate::web::subject::web_subject::WebSubject;

use super::{
    authc::{authentication_error::AuthenticationError, authentication_token::AuthenticationToken},
    authz::authorization_error::AuthorizationError,
    session::Session,
    util::object::Object,
};

#[async_trait]
pub trait Subject
where
    Self: Send + Sync,
    Self: DynClone,
    Self: Any,
{
    // === 身份相关 ===
    async fn get_principal(&self) -> Option<&Object>;
    async fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>>;

    // === 认证状态 ===
    async fn is_authenticated(&self) -> bool;
    async fn is_remembered(&self) -> bool;

    // === 授权（权限）===
    async fn is_permitted(&self, permission: &str) -> bool;
    async fn is_permitted_all(&self, permissions: &[&str]) -> bool;
    async fn check_permission(&self, permission: &str) -> Result<(), AuthorizationError>;
    async fn check_permissions(&self, permissions: &[&str]) -> Result<(), AuthorizationError>;

    // === 授权（角色）===
    async fn has_role(&self, role: &str) -> bool;
    async fn has_all_roles(&self, roles: &[&str]) -> bool;
    async fn check_role(&self, role: &str) -> Result<(), AuthorizationError>;
    async fn check_roles(&self, roles: &[&str]) -> Result<(), AuthorizationError>;

    // === 会话 ===
    fn get_session(&self) -> Option<&Arc<dyn Session>>;
    async fn get_session_or_create(&mut self, create: bool) -> Option<Arc<dyn Session>>;

    // === 登录/登出 ===
    async fn login(
        &mut self,
        token: &dyn AuthenticationToken,
        #[cfg(feature = "web")] req: &mut dyn HttpRequest,
        #[cfg(feature = "web")] resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError>;
    async fn logout(
        &mut self,
        #[cfg(feature = "web")] req: &mut dyn HttpRequest,
        #[cfg(feature = "web")] resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;

    async fn run_as(
        &mut self,
        principals: &Arc<dyn PrincipalCollection>,
    ) -> Result<(), IllegalStateError>;
    async fn is_run_as(&self) -> bool;
    async fn get_previous_principals(&self) -> Option<Arc<dyn PrincipalCollection>>;
    async fn release_run_as(&mut self) -> Option<&dyn PrincipalCollection>;
}

next_web_core::clone_trait_object!(Subject);
