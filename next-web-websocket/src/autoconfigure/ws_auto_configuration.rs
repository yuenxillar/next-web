use next_web_context::ApplicationContextExt;
use std::{error::Error, sync::Arc};

use axum::{Router, routing::any};
use next_web_core::{
    ApplicationContext, Ordered, async_trait,
    traits::{apply_router::ApplyRouter, config::auto_configuration::AutoConfiguration},
};
use next_web_macros::singleton;

use crate::{
    autoconfigure::ws_properties::WebSocketProperties,
    config::{
        default_ws_handler_registry::DefaultWebSocketHandlerRegistry,
        ws_configurer::WebSocketConfigurer,
    },
    server::{
        handshake_interceptor::HandshakeInterceptor,
        support::{
            origin_handshake_interceptor::OriginHandshakeInterceptor, ws_context::WebSocketContext,
        },
    },
    ws_handle::websocket_handle,
};

/// Auto-configuration for  Websocket.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct WebsocketAutoConfiguration {
    /// Websocket properties.
    pub web_socket_properties: WebSocketProperties,

    #[autowired(vec)]
    configurers: Vec<Box<dyn WebSocketConfigurer>>,
}

impl WebsocketAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for WebsocketAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        for configurer in self.configurers.iter_mut() {
            configurer.register_websocket_handlers(ctx, &mut registry);
        }

        let mut interceptors = ctx.resolve_by_type::<Arc<dyn HandshakeInterceptor>>();
        for registration in registry
            .registrations
            .iter()
            .filter(|s| !(s.allowed_origin_patterns().is_empty() && s.allowed_origins().is_empty()))
        {
            interceptors.push(Arc::new(OriginHandshakeInterceptor::new(
                registration.allowed_origins(),
                registration.allowed_origin_patterns(),
            )));
        }

        let ws_context = WebSocketContext::new(
            self.web_socket_properties.clone(),
            registry.handler_mapping(),
            interceptors,
        );

        let instance = Box::new(WebsocketApplyRouter { ws_context });
        ctx.insert_singleton_with_name::<Box<dyn ApplyRouter>>(instance, "websocketApplyRouter");
        Ok(())
    }
}

impl Ordered for WebsocketAutoConfiguration {
    fn order(&self) -> i32 {
        100
    }
}

#[derive(Clone)]
pub(crate) struct WebsocketApplyRouter {
    ws_context: WebSocketContext,
}

impl ApplyRouter for WebsocketApplyRouter {
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
        self.ws_context
            .handler_mapping()
            .paths()
            .iter()
            .fold(Router::new(), |router, path| {
                router.route(path.as_str(), any(websocket_handle))
            })
            .with_state(Arc::new(std::mem::take(&mut self.ws_context)))
    }
}
