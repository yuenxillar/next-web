pub mod access;
pub mod auth;
pub mod authorization;
pub mod config;
pub mod core;
pub mod crypto;
pub mod permission;
pub mod use_router;
pub mod web;



#[derive(Clone)]
#[rudi_dev::Singleton(binds = [Self::into_use_router])]
pub struct AuthenticationUseRouter;

impl AuthenticationUseRouter {
    fn into_use_router(self) -> Box<dyn next_web_core::traits::use_router::UseRouter> {
        Box::new(self)
    }
}

impl next_web_core::traits::use_router::UseRouter for AuthenticationUseRouter {
    fn use_router(
        &self,
        mut router: axum::Router,
        ctx: &mut next_web_core::ApplicationContext,
    ) -> axum::Router {
        let auth_service =
            ctx.resolve_by_type::<std::sync::Arc<
                dyn permission::service::authentication_service::AuthenticationService,
            >>();

        if let Some(service) = auth_service.last() {
            let web_security_configure = ctx.resolve_by_type::<Box<dyn crate::core::web_security_configure::WebSecurityConfigure>>();
            let http_security = web_security_configure
                .last()
                .map(|var| var.configure())
                .unwrap_or_default();
            let user_authorization_manager =
                permission::manager::user_authorization_manager::UserAuthenticationManager::new(
                    service.clone(),
                    http_security,
                );
            router = router.layer(axum::middleware::from_fn_with_state(
                user_authorization_manager,
                permission::middleware::request_auth_middleware::request_auth_middleware,
            ));
        }
        router
    }
}
