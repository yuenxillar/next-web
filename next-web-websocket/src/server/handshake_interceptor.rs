use std::collections::HashMap;

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
/// 握手拦截器 trait
///
/// 可以在 WebSocket 握手前后执行自定义逻辑，例如：
/// - 身份验证
/// - 请求/响应检查
/// - 向 WebSocket 会话传递属性
#[async_trait]
pub trait HandshakeInterceptor
where
    Self: Send + Sync,
{
    /// Called before handshake processing
    ///
    /// 在握手处理之前调用
    async fn before_handshake(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError>;
}
