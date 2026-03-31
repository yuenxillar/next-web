use std::sync::Arc;

use crate::{
    config::ws_handler_registration::WebSocketHandlerRegistration, ws_handler::WebSocketHandler,
};

///  Provides methods for configuring {@link WebSocketHandler} request mappings.
pub trait WebSocketHandlerRegistry {
    /// Configure a WebSocketHandler at the specified URL paths.

    fn add_handler<'a>(
        &'a mut self,
        ws_handler: Arc<dyn WebSocketHandler>,
        paths: Vec<String>,
    ) -> &'a mut dyn WebSocketHandlerRegistration;
}
