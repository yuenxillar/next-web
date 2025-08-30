use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::Uri,
    response::IntoResponse,
    routing::any,
    Router,
};
use next_web_core::{traits::apply_router::ApplyRouter, ApplicationContext};
use rudi_dev::Singleton;

use crate::{
    handler::websocket_handler::{handle_socket, WebSocketHandler},
    model::ws_context::WebSocketContext,
};

/// WebSocket路由器，负责将WebSocket处理器注册到指定路径
///
/// WebSocket router that registers WebSocket handlers to specified paths
///
/// # 特性/Features
/// - 自动将自身转换为`ApplyRouter` trait对象/Automatically converts itself into an `ApplyRouter` trait object
/// - 使用`Singleton`注解确保全局唯一实例/Annotated with `Singleton` to ensure global uniqueness
#[Singleton(binds = [Self::into_router])]
#[derive(Clone)]
pub(crate) struct WSApplyRouter;

impl WSApplyRouter {
    /// 将WSRouter转换为`ApplyRouter` trait对象
    ///
    /// Converts WSRouter into an `ApplyRouter` trait object
    ///
    /// # 返回值/Returns
    /// - 装箱的`ApplyRouter` trait对象/Boxed `ApplyRouter` trait object
    fn into_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl ApplyRouter for WSApplyRouter {
    /// 实现`ApplyRouter` trait，构建WebSocket路由
    ///
    /// Implements `ApplyRouter` trait to build WebSocket routes
    ///
    /// # 参数/Parameters
    /// - `ctx`: 应用上下文，用于解析依赖项/Application context for dependency resolution
    ///
    /// # 返回值/Returns
    /// - 配置好的axum路由器/Configured axum router
    fn router(&self, ctx: &mut ApplicationContext) -> axum::Router {
        let mut router = Router::new();
        let mut context = ctx.resolve::<WebSocketContext>();
        let handlers = ctx.resolve_by_type::<Arc<dyn WebSocketHandler>>();

        for item in handlers.iter() {
            for path in item.paths() {
                router = router.route(path, any(ws_handler));
                context.add_handler(path, item.clone());
            }
        }
        router.with_state(Arc::new(context))
    }
}

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
/// WebSocket升级处理器，处理HTTP到WebSocket的协议升级
///
/// WebSocket upgrade handler that processes HTTP to WebSocket protocol upgrade
///
/// # 参数/Parameters
/// - `ws`: WebSocket升级请求/WebSocket upgrade request
/// - `ctx`: WebSocket上下文，包含处理器和配置/WebSocket context with handlers and configurations
/// - `addr`: 客户端地址/Client address
/// - `uri`: 请求URI/Request URI
/// - `header`: HTTP请求头/HTTP request headers
///
/// # 返回值/Returns
/// - 实现`IntoResponse`的响应对象/Response object implementing `IntoResponse`
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<Arc<WebSocketContext>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    uri: Uri,
    header: axum::http::HeaderMap,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    let properties = ctx.properties();
    let path = uri.path().to_string();
    ws.max_message_size(properties.max_msg_size().unwrap_or(64 << 20))
        .max_write_buffer_size(properties.max_write_buffer_size().unwrap_or(usize::MAX))
        .on_upgrade(move |socket| handle_socket(socket, ctx, addr, path, header))
}
