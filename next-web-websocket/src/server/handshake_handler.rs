use std::collections::HashMap;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::ws_handler::WebSocketHandler;

/// Contract for processing a WebSocket handshake request.
#[async_trait]
pub trait HandshakeHandler {
    /// Initiate the handshake.
    async fn do_handshake(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ws_handler: &dyn WebSocketHandler,
        attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError>;
}
