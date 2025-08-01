use std::sync::Arc;

use next_web_core::interface::apply_router::ApplyRouter;
use rudi_dev::Singleton;

use crate::core::web_security_configure::WebSecurityConfigure;
use crate::permission::middleware::request_auth_middleware::request_auth_middleware;
use crate::permission::{
    manager::user_authorization_manager::UserAuthenticationManager,
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
        let auth_service = ctx.resolve_by_type::<Arc<dyn AuthenticationService>>();

        router = router.route("/test3/666", axum::routing::get(async || "666"));
        if let Some(service) = auth_service.last() {
            let web_security_configure = ctx.resolve_by_type::<Box<dyn WebSecurityConfigure>>();
            let http_security = web_security_configure
                .last()
                .map(|var| var.configure())
                .unwrap_or_default();
            let user_authorization_manager =
                UserAuthenticationManager::new(service.clone(), http_security);
            router = router.layer(axum::middleware::from_fn_with_state(
                user_authorization_manager,
                request_auth_middleware,
            ));
        }

        router
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {
        let mut router = matchit::Router::new();

        router.insert("/test33", 1).unwrap();
        router.insert("/test33/{*param}", 2).unwrap();
        router.insert("/test33/666", 3).unwrap();
        // router.insert("/test33/{*index}", 3).unwrap();
        // router.insert("/test33/{*index1}", 4).unwrap();
        // router.insert("/test33/{*.css}", 5).unwrap();

        println!("{:?}", router.at("/test33"));
        println!(
            "{:?}",
            router.at("/test33/666.js").map(|var| var.params.get("param"))
        );
        println!(
            "{:?}",
            router.at("/test33/777.css").map(|var| var.params.get("param"))
        );
        println!(
            "{:?}",
            router.at("/test33/666").map(|var| var.value)
        );

        println!("node: {:?}", router);
    }
}
