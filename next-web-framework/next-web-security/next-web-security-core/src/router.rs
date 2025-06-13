use std::sync::Arc;

use next_web_core::core::router::ApplyRouter;
use rudi_dev::Singleton;

use crate::permission::{
    manager::user_authorization_manager::UserAuthenticationManager,
    middleware::request_auth_middleware::request_auth_middleware,
    service::authentication_service::AuthenticationService,
};

#[derive(Clone)]
#[Singleton(binds = [Self::into_apply_router])]
pub struct AuthenticationRouter;

impl AuthenticationRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl ApplyRouter for AuthenticationRouter {
    fn order(&self) -> u32 {
        u32::MAX
    }

    fn router(&self, ctx: &mut next_web_core::ApplicationContext) -> axum::Router {
        let mut router = axum::Router::new();
        let auth_service = ctx.resolve_option::<Arc<dyn AuthenticationService>>();
        if let Some(service) = auth_service {
            let user_authorization_manager = UserAuthenticationManager::new(service);
            router = router.route_layer(axum::middleware::from_fn_with_state(
                user_authorization_manager,
                request_auth_middleware,
            ));
        }
        router
    }
}
