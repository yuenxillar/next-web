//! Configuration for the OAuth 2.1 Authorization Server support.
//!
//! The configuration contributes the security filter chain the endpoints of the
//! authorization server are served by. The chain is contributed when the
//! application registered the clients that are registered with the server, and
//! when it did not declare a chain of its own: the chain of the application
//! then holds the endpoints of the server as well.
//!
//! An application that declares its own chain applies the endpoints itself,
//! with the `oauth2_authorization_server` configuration of its HTTP security.

use std::{any::TypeId, sync::Arc};

use next_web_macros::auto_configuration;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        web::{builders::HttpSecurity, configurers::AuthorizeHttpRequestsConfigurer},
    },
    oauth2::server::authorization::{AuthorizationServerSettings, RegisteredClientRepository},
    web::security_filter_chain::SecurityFilterChain,
};
use next_web_context::ApplicationContextExt;
use next_web_core::ApplicationContext;

/// Configuration for the OAuth 2.1 Authorization Server support.
#[derive(Debug, Clone)]
pub struct OAuth2AuthorizationServerConfiguration;

/// Configures the security filter chain of the authorization server.
///
/// The chain is contributed with the highest precedence, and it is the first
/// chain that matches a request. An application that declares authorization
/// rules of its own keeps them, and the endpoints of the authorization server
/// are served by the same chain.
#[auto_configuration]
impl OAuth2AuthorizationServerConfiguration {
    /// Builds the security filter chain of the authorization server.
    ///
    /// # Arguments
    ///
    /// * `http_security` - The HTTP security the chain is built from.
    #[provider(
        name = "authorizationServerSecurityFilterChain",
        order = i32::MIN,
        conditional = [Self::on_authorization_server]
    )]
    fn authorization_server_security_filter_chain(
        http_security: &mut HttpSecurity,
    ) -> Arc<dyn SecurityFilterChain> {
        http_security.oauth2_authorization_server(|_| {});

        // An authorization the application declared is not extended by the
        // default one, because its `any_request` would be configured after the
        // one of the application.
        if http_security
            .configurer::<AuthorizeHttpRequestsConfigurer<HttpSecurity>>()
            .is_none()
        {
            http_security.authorize_http_requests(|authorize| {
                authorize
                    .request_matchers(&[
                        AuthorizationServerSettings::DEFAULT_AUTHORIZATION_SERVER_METADATA_ENDPOINT,
                        AuthorizationServerSettings::DEFAULT_JWK_SET_ENDPOINT,
                    ])
                    .permit_all()
                    .any_request()
                    .authenticated();
            });
        }

        Arc::new(http_security.build())
    }

    /// Returns whether the authorization server chain is contributed.
    ///
    /// The chain is contributed when the application registered the clients of
    /// the server, so that the server has a client it can authenticate, and
    /// when the application did not declare a chain of its own.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of the application.
    fn on_authorization_server(ctx: &dyn ApplicationContext) -> bool {
        let has_registered_clients =
            ctx.contains_dyn_singleton_with_default_name::<Arc<dyn RegisteredClientRepository>>();
        let has_security_filter_chain = !ctx
            .keys_of_type(TypeId::of::<Arc<dyn SecurityFilterChain>>())
            .is_empty();

        has_registered_clients && !has_security_filter_chain
    }
}
