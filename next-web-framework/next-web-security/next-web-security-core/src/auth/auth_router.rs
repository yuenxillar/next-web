use std::sync::Arc;

use axum::Router;
use next_web_core::core::apply_router::ApplyRouter;
use rudi_dev::Singleton;

use crate::{
    auth::middleware::request_auth_middleware::request_auth_middleware,
    core::{http_security::HttpSecurity, web_security_configure::WebSecurityConfigure},
    permission::{
        manager::user_authorization_manager::UserAuthenticationManager,
        service::authentication_service::AuthenticationService,
    },
};

#[Singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct AuthRouter;

impl AuthRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl ApplyRouter for AuthRouter {
    fn order(&self) -> u32 {
        u32::MAX
    }

    fn router(&self, ctx: &mut next_web_core::ApplicationContext) -> axum::Router {
        let authentication_service = ctx.resolve_option::<Box<dyn AuthenticationService>>();

        let security_configure = ctx.resolve_option::<Box<dyn WebSecurityConfigure>>();
        let user_auth_manager = UserAuthenticationManager::new(
            authentication_service,
            if let Some(security) = security_configure {
                security.configure()
            } else {
                HttpSecurity::new()
            },
        );
        Router::new()
    }
}
