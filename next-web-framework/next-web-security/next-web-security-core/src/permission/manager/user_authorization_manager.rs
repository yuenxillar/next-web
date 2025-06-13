use std::sync::Arc;

use crate::auth::models::login_type::LoginType;
use crate::permission::{
    models::permission_group::{CombinationMode, PermissionGroup},
    service::authentication_service::AuthenticationService,
};

#[derive(Clone)]
pub struct UserAuthenticationManager
{
    // options: UserAuthorizationOptions,
    authentication_service: Arc<dyn AuthenticationService>,
}

impl UserAuthenticationManager
{
    pub fn new(
        // options: UserAuthorizationOptions,
        authentication_service: Arc<dyn AuthenticationService>,
    ) -> Self {
        Self {
            // options,
            authentication_service,
        }
    }

    // pub fn options(&self) -> &UserAuthorizationOptions {
    //     &self.options
    // }

    pub fn authentication_service(&self) -> &Arc<dyn AuthenticationService> {
        &self.authentication_service
    }
}

impl UserAuthenticationManager
{
    pub async fn pre_authorize<'a>(
        &self,
        user_id: &'a String,
        login_type: &'a LoginType,
        auth_group: &'a PermissionGroup,
    ) -> bool {
        if auth_group.is_combination() {
            if auth_group.combination_valid() {
                return true;
            }
        }

        let roles = auth_group.roles();
        let permissions = auth_group.permissions();
        let binding = auth_group.mode();
        let mode = binding.as_ref();

        if roles.is_none() && permissions.is_none() {
            return true;
        }

        let user_roles = self
            .authentication_service
            .user_role(user_id, login_type)
            .await;

        // Mode  And or Or
        // tips:
        // 1. And mode: if user has all roles and permissions, return true
        // 2. Or mode: if user has any roles or permissions, return true
        let role_flag = auth_group.match_value(
            roles.unwrap_or_default(),
            user_roles,
            mode,
        );
        if let Some(var1) = mode {
            if var1 != &CombinationMode::Or {
                if !role_flag {
                    return false;
                }
            }
        } else {
            if !role_flag {
                return false;
            }
        }

        let user_permissions = self
            .authentication_service
            .user_permission(user_id, login_type)
            .await;

        let permission_flag = auth_group.match_value(
            permissions.unwrap_or_default(),
            user_permissions,
            mode,
        );

        if role_flag && permission_flag {
            return true;
        }
        false
    }
}
