use std::sync::Arc;

use next_web_core::models::any_matcher::AnyMatcher;

use crate::auth::models::login_type::LoginType;
use crate::config::web::http_security::HttpSecurity;
use crate::permission::{
    model::permission_group::PermissionGroup,
    service::authentication_service::AuthenticationService,
};

#[derive(Clone)]
pub struct UserAuthenticationManager {
    // options: UserAuthorizationOptions,
    http_security: Arc<HttpSecurity>,
    router: Arc<AnyMatcher<PermissionGroup>>,
    authentication_service: Arc<dyn AuthenticationService>,
}

impl UserAuthenticationManager {
    pub fn new(
        // options: UserAuthorizationOptions,
        authentication_service: Arc<dyn AuthenticationService>,
        http_security: HttpSecurity,
    ) -> Self {
        let router = Self::build_router(&http_security);
        Self {
            // options,
            authentication_service,
            http_security: Arc::new(http_security),
            router: Arc::new(router),
        }
    }

    // pub fn options(&self) -> &UserAuthorizationOptions {
    //     &self.options
    // }

    pub fn authentication_service(&self) -> &Arc<dyn AuthenticationService> {
        &self.authentication_service
    }

    pub fn router(&self) -> &Arc<AnyMatcher<PermissionGroup>> {
        &self.router
    }

    pub fn http_security(&self) -> &Arc<HttpSecurity> {
        &self.http_security
    }

    fn build_router(http_security: &HttpSecurity) -> AnyMatcher<PermissionGroup> {
        let mut route_resources = AnyMatcher::new();
        http_security
            .any_match
            .clone()
            .into_iter()
            .for_each(|item| route_resources.insert(item.0, item.1).unwrap());

        http_security
            .not_match
            .clone()
            .into_iter()
            .for_each(|item| route_resources.insert(item, Default::default()).unwrap());

        route_resources
    }
}

impl UserAuthenticationManager {
    pub async fn pre_authorize<'a>(
        &self,
        user_id: &'a String,
        login_type: &'a LoginType,
        auth_group: &'a PermissionGroup,
    ) -> bool {
        let roles = auth_group.get_roles();
        let permissions = auth_group.get_permissions();
        // let mode = auth_group.get_mode();

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
        let role_flag = auth_group.match_value(roles, user_roles.as_ref());

        if !role_flag {
            return false;
        }

        let user_permissions = self
            .authentication_service
            .user_permission(user_id, login_type)
            .await;

        let permission_flag = auth_group.match_value(permissions, user_permissions.as_ref());

        if role_flag && permission_flag {
            return true;
        }
        false
    }
}
