use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::Uri,
    response::IntoResponse,
    routing::any,
    Router,
};
use next_web_core::{core::router::ApplyRouter, ApplicationContext};
use rudi_dev::Singleton;

use super::{
    handle_socket::handle_socket, handler::WebSocketHandler, ws_context::WebSocketContext,
};

#[Singleton(binds = [Self::into_router])]
#[derive(Clone)]
pub(crate) struct WSRouter;

impl WSRouter {
    fn into_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl ApplyRouter for WSRouter {
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
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<Arc<WebSocketContext>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    uri: Uri,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    let properties = ctx.properties();
    let path = uri.path().to_string();
    ws.max_message_size(properties.max_msg_size().unwrap_or(64 << 20))
        .max_write_buffer_size(properties.max_write_buffer_size().unwrap_or(usize::MAX))
        .on_upgrade(move |socket| handle_socket(socket, ctx, addr, path))
}
