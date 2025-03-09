// #[cfg(feature = "redis")]
use crate::{
    autoconfigure::context::user_authorization_options_properties::UserAuthorizationOptions,
    security::{
        auth_group::{AuthGroup, CombinationMode}, authorization_service::AuthorizationService, login_type::LoginType,
        user_permission_resource::UserPermissionResource,
    },
};

#[derive(Clone)]
pub struct UserAuthorizationManager<T>
where
    T: AuthorizationService<Vec<String>> + Clone,
{
    options: UserAuthorizationOptions,
    authorization_service: T,
    user_permission_resource: UserPermissionResource,
}

impl<T> UserAuthorizationManager<T>
where
    T: AuthorizationService<Vec<String>> + Clone,
{
    pub fn new(
        options: UserAuthorizationOptions,
        authorization_service: T,
        user_permission_resource: UserPermissionResource,
    ) -> Self {
        Self {
            options,
            authorization_service,
            user_permission_resource,
        }
    }

    pub fn options(&self) -> &UserAuthorizationOptions {
        &self.options
    }

    pub fn authorization_service(&self) -> &T {
        &self.authorization_service
    }

    pub fn get_permission(&self, method: &axum::http::Method, path: &str) -> Option<&AuthGroup> {
        self.user_permission_resource.get_permission(method, path)
    }
}

impl<T> UserAuthorizationManager<T>
where
    T: AuthorizationService<Vec<String>> + Clone,
{
    pub async fn pre_authorize(
        &self,
        user_id: &String,
        auth_group: &AuthGroup,
        login_type: &LoginType,
    ) -> bool {
        if user_id.is_empty() {
            return false;
        }

        if auth_group.is_combination() {
            if auth_group.combination_valid() {
                return true;
            }
        }

        let roles = auth_group.roles().unwrap_or_default();
        let permissions = auth_group.permissions().unwrap_or_default();
        let bing = auth_group.mode();
        let mode = bing.as_ref();

        if roles.is_empty() && permissions.is_empty() {
            return true;
        }

        let user_roles = self
            .authorization_service
            .user_role(user_id, login_type)
            .await;

        // Mode  And or Or
        // tips:
        // 1. And mode: if user has all roles and permissions, return true
        // 2. Or mode: if user has any roles or permissions, return true
        let role_flag = auth_group.match_value(roles, user_roles, mode);
        if let Some(_model ) = mode {
            if _model != &CombinationMode::Or {
                if !role_flag {
                    return false;
                }
            }
        }else {
            if !role_flag {
                return false;
            }
        }
        
        let user_permissions = self
            .authorization_service
            .user_permission(user_id, login_type)
            .await;

        let permission_flag =
            auth_group.match_value(permissions, user_permissions, mode);

        if role_flag && permission_flag {
            return true;
        }
        false
    }

    pub async fn verify_token(&self, token: &String, login_type: &LoginType) -> bool {
        self.authorization_service
            .verify_token(token, login_type)
            .await
    }
}
