use crate::{
    core::http_security::HttpSecurity,
    permission::{
        models::permission_group::{CombinationMode, PermissionGroup},
        service::authentication_service::AuthenticationService,
    },
};

#[derive(Clone)]
pub struct UserAuthenticationManager<S>
where
    S: AuthenticationService,
{
    authentication_service: S,
    pub(crate) http_security: HttpSecurity,
}

impl<S> UserAuthenticationManager<S>
where
    S: AuthenticationService,
{
    pub fn new(authentication_service: S, http_security: HttpSecurity) -> Self {
        Self {
            authentication_service,
            http_security,
        }
    }

    pub fn authentication_service(&self) -> &S {
        &self.authentication_service
    }
}

impl<S> UserAuthenticationManager<S>
where
    S: AuthenticationService,
{
    pub async fn pre_authorize(
        &self,
        req_headers: &axum::http::HeaderMap,
        permission_group: &PermissionGroup,
    ) -> bool {
        if permission_group.valid() {
            return true;
        }

        let roles = permission_group.get_roles();
        let permissions = permission_group.get_permissions();
        let mode = permission_group.get_mode();

        let user_id = self.authentication_service.id(req_headers);
        let login_type = self.authentication_service.login_type(req_headers);

        let user_roles = self
            .authentication_service
            .user_role(&user_id, &login_type)
            .await
            .map(|s| s.iter().map(|s| s.to_string()).collect::<Vec<_>>());

        // Mode  And or Or
        // tips:
        // 1. And mode: if user has all roles and permissions, return true
        // 2. Or mode: if user has any roles or permissions, return true
        let role_flag = permission_group.match_value(roles, user_roles.as_ref());
        if *mode == CombinationMode::And {
            if !role_flag {
                return false;
            }
        } else {
            if role_flag {
                return true;
            }
        }

        let user_permissions = self
            .authentication_service
            .user_permission(&user_id, &login_type)
            .await
            .map(|s| s.iter().map(|s| s.to_string()).collect::<Vec<_>>());

        let permission_flag = permission_group.match_value(permissions, user_permissions.as_ref());

        if role_flag && permission_flag {
            return true;
        }
        false
    }
}
