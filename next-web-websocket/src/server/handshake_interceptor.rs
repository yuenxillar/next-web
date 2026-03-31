use std::collections::HashMap;
use std::error::Error;

use next_web_core::anys::any_value::AnyValue;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;

use crate::ws_handler::WebSocketHandler;

/// 握手拦截器 trait
///
/// 可以在 WebSocket 握手前后执行自定义逻辑，例如：
/// - 身份验证
/// - 请求/响应检查
/// - 向 WebSocket 会话传递属性
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use your_crate::{
///     HandshakeInterceptor, HttpRequest, HttpResponse,
///     WebSocketHandler, WebSocketSession
/// };
///
/// struct AuthInterceptor {
///     allowed_tokens: Vec<String>,
/// }
///
/// #[async_trait]
/// impl HandshakeInterceptor for AuthInterceptor {
///     async fn before_handshake(
///         &self,
///         request: &dyn HttpRequest,
///         response: &mut dyn HttpResponse,
///         handler: &dyn WebSocketHandler,
///         attributes: &mut HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
///     ) -> Result<bool, Box<dyn std::error::Error>> {
///         // 检查认证 token
///         if let Some(token) = request.headers().get("Authorization") {
///             if self.allowed_tokens.contains(token) {
///                 attributes.insert("authenticated".to_string(), Box::new(true));
///                 return Ok(true);
///             }
///         }
///         
///         response.set_status(401);
///         Ok(false)
///     }
///     
///     async fn after_handshake(
///         &self,
///         request: &dyn HttpRequest,
///         response: &dyn HttpResponse,
///         handler: &dyn WebSocketHandler,
///         exception: Option<&Box<dyn std::error::Error>>,
///     ) -> Result<(), Box<dyn std::error::Error>> {
///         if let Some(e) = exception {
///             eprintln!("Handshake failed: {}", e);
///         } else {
///             println!("Handshake successful for {}", request.uri());
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait HandshakeInterceptor: Send + Sync {
    /// 在握手处理之前调用
    ///
    /// # Arguments
    /// * `request` - 当前 HTTP 请求
    /// * `response` - 当前 HTTP 响应
    /// * `handler` - 目标 WebSocket 处理器
    /// * `attributes` - 属性映射，用于传递给 WebSocket 会话
    ///
    /// # Returns
    /// * `Ok(true)` - 继续握手
    /// * `Ok(false)` - 中止握手
    /// * `Err(_)` - 发生错误，握手失败
    async fn before_handshake(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        handler: &dyn WebSocketHandler,
        attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError>;

    /// 在握手完成之后调用
    ///
    /// # Arguments
    /// * `request` - 当前 HTTP 请求
    /// * `response` - 当前 HTTP 响应（状态码和头信息反映握手结果）
    /// * `handler` - 目标 WebSocket 处理器
    /// * `error` - 握手中发生的异常，如果为 `None` 表示握手成功
    async fn after_handshake(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        handler: &dyn WebSocketHandler,
        error: Option<&dyn Error>,
    ) -> Result<(), BoxError>;
}
