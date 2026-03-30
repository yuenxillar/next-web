use next_web_core::{ApplicationContext, DynClone, clone_trait_object};

use crate::ws::config::ws_handler_registry::WebSocketHandlerRegistry;

/// Defines callback methods to configure the WebSocket request handling
pub trait WebSocketConfigurer
where
    Self: DynClone,
    Self: Send + Sync,
{
    /// Register WebSocketHandler including SockJS fallback options if desired.
    fn register_web_socket_handlers(
        &mut self,
        ctx: &mut ApplicationContext,
        registry: &mut dyn WebSocketHandlerRegistry,
    );
}

clone_trait_object!(WebSocketConfigurer where Self: Send + Sync);