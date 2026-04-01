use std::{any::Any, collections::HashMap};

use next_web_core::anys::any_value::AnyValue;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;

///
/// Handshake interceptor trait
/// Custom logic can be executed before and after WebSocket handshake, for example:
/// - Identity verification
/// - Request/Response Check
/// - Pass properties to WebSocket session
#[async_trait]
pub trait HandshakeInterceptor
where
    Self: Send + Sync + Any,
{
    /// Called before handshake processing
    async fn before_handshake(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError>;
}
