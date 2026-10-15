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
    sockjs::{router::SockJsRouter, service::SockJsService},
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
        let sockjs_interceptors = interceptors.to_vec();

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

        let sockjs_services = registry.sockjs_services(&sockjs_interceptors);

        let ws_context = WebSocketContext::new(
            self.web_socket_properties.clone(),
            registry.handler_mapping(),
            interceptors,
        );

        let instance = Box::new(WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        });
        ctx.insert_singleton_with_name::<Box<dyn ApplyRouter>>(instance, "websocketApplyRouter");

        Ok(())
    }
}

impl Ordered for WebsocketAutoConfiguration {}

#[derive(Clone)]
pub(crate) struct WebsocketApplyRouter {
    ws_context: WebSocketContext,
    sockjs_services: Vec<(Vec<String>, Arc<dyn SockJsService>)>,
}

impl ApplyRouter for WebsocketApplyRouter {
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
        self.build_router()
    }
}

impl WebsocketApplyRouter {
    fn build_router(&mut self) -> axum::Router {
        let websocket_router = self
            .ws_context
            .handler_mapping()
            .paths()
            .iter()
            .fold(Router::new(), |router, path| {
                router.route(path.as_str(), any(websocket_handle))
            })
            // Cloned rather than taken. Taking it consumed the context, so a
            // router built a second time would have had no handlers, no
            // interceptors and default properties.
            .with_state(Arc::new(self.ws_context.clone()));

        self.sockjs_services
            .iter()
            .fold(websocket_router, |router, (paths, service)| {
                paths
                    .iter()
                    .fold(router, |router, path| router.sockjs(path, service.clone()))
            })
    }
}

impl Ordered for WebsocketApplyRouter {}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, error::Error, sync::Arc};

    use axum::{
        body::{Body, to_bytes},
        extract::ws::{CloseFrame, Message},
        http::{Request, StatusCode},
    };
    use futures::{SinkExt, StreamExt};
    use next_web_core::http::HttpResponse;
    use next_web_core::{any::AnyValue, async_trait, error::BoxError};
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    use crate::{
        autoconfigure::ws_properties::WebSocketProperties,
        config::{
            default_ws_handler_registry::DefaultWebSocketHandlerRegistry,
            ws_handler_registry::WebSocketHandlerRegistry,
        },
        server::{
            handshake_interceptor::HandshakeInterceptor, support::ws_context::WebSocketContext,
        },
        ws_handler::{WSResult, WebSocketHandler},
    };

    use super::WebsocketApplyRouter;

    struct TestHandler;

    struct RejectSockJsRequest;

    #[async_trait]
    impl HandshakeInterceptor for RejectSockJsRequest {
        async fn before_handshake(
            &self,
            _request: &mut dyn next_web_core::http::HttpRequest,
            response: &mut dyn HttpResponse,
            _attributes: &mut HashMap<String, AnyValue>,
        ) -> Result<bool, BoxError> {
            response.set_status_code(StatusCode::FORBIDDEN);
            Ok(false)
        }
    }

    #[async_trait]
    impl WebSocketHandler for TestHandler {
        async fn on_open(
            &self,
            session: &crate::server::support::ws_session::WebSocketSession,
        ) -> WSResult<()> {
            session
                .send_message(Message::Text("welcome".into()))
                .await
                .map_err(|_| "failed to send welcome message")?;
            Ok(())
        }

        async fn on_message(
            &self,
            session: &crate::server::support::ws_session::WebSocketSession,
            message: Message,
        ) -> WSResult<()> {
            if let Message::Text(text) = message {
                session
                    .send_message(Message::Text(format!("echo: {text}").into()))
                    .await
                    .map_err(|_| "failed to send echo message")?;
            }
            Ok(())
        }

        async fn on_error(
            &self,
            _session: &crate::server::support::ws_session::WebSocketSession,
            _error: Box<dyn Error + Send + Sync>,
        ) -> WSResult<()> {
            Ok(())
        }

        async fn on_close(
            &self,
            _session: &crate::server::support::ws_session::WebSocketSession,
            _close: Option<CloseFrame>,
        ) -> WSResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn sockjs_registration_is_reachable_through_the_application_router() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_websocket_enabled(false)
            .set_allowed_origins(vec!["https://client.example".to_string()])
            .set_heartbeat_time(30);

        let sockjs_services = registry.sockjs_services(&[]);
        assert_eq!(sockjs_services.len(), 1);
        assert_eq!(sockjs_services[0].0, vec!["/events"]);

        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let info = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(info.status(), StatusCode::OK);
        let info_body = to_bytes(info.into_body(), usize::MAX).await.unwrap();
        let info_json: serde_json::Value = serde_json::from_slice(&info_body).unwrap();
        assert_eq!(info_json["websocket"], false);

        let cors_info = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events/info")
                    .header("origin", "https://client.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cors_info.status(), StatusCode::OK);
        assert_eq!(
            cors_info
                .headers()
                .get("access-control-allow-origin")
                .unwrap(),
            "https://client.example"
        );
        assert_eq!(
            cors_info
                .headers()
                .get("access-control-allow-credentials")
                .unwrap(),
            "true"
        );

        let preflight = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/events/000/session-cors/xhr_send")
                    .header("origin", "https://client.example")
                    .header("access-control-request-method", "POST")
                    .header("access-control-request-headers", "content-type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preflight.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            preflight
                .headers()
                .get("access-control-allow-methods")
                .unwrap(),
            "POST, OPTIONS"
        );

        let xhr_preflight = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/events/000/session-cors/xhr")
                    .header("origin", "https://client.example")
                    .header("access-control-request-method", "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(xhr_preflight.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            xhr_preflight
                .headers()
                .get("access-control-allow-methods")
                .unwrap(),
            "GET, POST, OPTIONS"
        );

        let streaming_preflight = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/events/000/session-cors/xhr_streaming")
                    .header("origin", "https://client.example")
                    .header("access-control-request-method", "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(streaming_preflight.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            streaming_preflight
                .headers()
                .get("access-control-allow-methods")
                .unwrap(),
            "GET, POST, OPTIONS"
        );

        let wrong_preflight = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/events/000/session-cors/xhr_streaming")
                    .header("origin", "https://client.example")
                    .header("access-control-request-method", "PUT")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_preflight.status(), StatusCode::METHOD_NOT_ALLOWED);

        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events/000/session-denied/xhr")
                    .header("origin", "https://attacker.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let poll = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events/000/session-1/xhr")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(poll.status(), StatusCode::OK);
        let poll_body = to_bytes(poll.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&poll_body[..], b"o\na[\"welcome\"]\n");

        let xhr_post_receive = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-post-receive/xhr")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(xhr_post_receive.status(), StatusCode::OK);
        let xhr_post_body = to_bytes(xhr_post_receive.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&xhr_post_body[..], b"o\na[\"welcome\"]\n");

        let send = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-1/xhr_send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"["hello"]"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(send.status(), StatusCode::NO_CONTENT);

        let next_poll = app.clone().oneshot(
            Request::builder()
                .uri("/events/000/session-1/xhr")
                .body(Body::empty())
                .unwrap(),
        );
        let next_poll = next_poll.await.unwrap();
        assert_eq!(next_poll.status(), StatusCode::OK);
        let next_poll_body = to_bytes(next_poll.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&next_poll_body[..], b"a[\"echo: hello\"]\n");

        let idle_poll = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/events/000/session-1/xhr")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(idle_poll.status(), StatusCode::OK);
        let idle_poll_body = to_bytes(idle_poll.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&idle_poll_body[..], b"h\n");

        let wrong_method = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/events/000/session-1/xhr_streaming")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);

        let invalid_transport = app
            .oneshot(
                Request::builder()
                    .uri("/events/000/session-1/jsonp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_transport.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn sockjs_xhr_streaming_and_send_fallback_roundtrip() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_websocket_enabled(false)
            .set_heartbeat_time(100);
        let sockjs_services = registry.sockjs_services(&[]);
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-stream/xhr_streaming")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let mut stream = response.into_body().into_data_stream();
        let prelude = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(prelude.len(), 2049);
        assert!(prelude[..2048].iter().all(|byte| *byte == b'h'));
        assert_eq!(prelude[2048], b'\n');

        let opening = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(&opening[..], b"o\na[\"welcome\"]\n");

        let send = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-stream/xhr_send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"["hello"]"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(send.status(), StatusCode::NO_CONTENT);

        let echo = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(&echo[..], b"a[\"echo: hello\"]\n");
    }

    #[tokio::test]
    async fn sockjs_xhr_polling_returns_before_the_heartbeat_deadline() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_websocket_enabled(false)
            .set_heartbeat_time(10_000);
        let sockjs_services = registry.sockjs_services(&[]);
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let initial = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-long-poll/xhr")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(initial.status(), StatusCode::OK);
        let initial_body = to_bytes(initial.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&initial_body[..], b"o\na[\"welcome\"]\n");

        let idle_poll = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-long-poll/xhr")
                    .body(Body::empty())
                    .unwrap(),
            ),
        )
        .await
        .expect("long-poll response headers should not wait for the heartbeat")
        .unwrap();
        assert_eq!(idle_poll.status(), StatusCode::OK);

        let body_task =
            tokio::spawn(async move { to_bytes(idle_poll.into_body(), usize::MAX).await.unwrap() });
        let send = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-long-poll/xhr_send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"["polling message"]"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(send.status(), StatusCode::NO_CONTENT);

        let body = tokio::time::timeout(std::time::Duration::from_secs(1), body_task)
            .await
            .expect("the poll should complete when a message arrives")
            .unwrap();
        assert_eq!(&body[..], b"a[\"echo: polling message\"]\n");
    }

    #[tokio::test]
    async fn with_sock_js_runs_registered_handshake_interceptors_on_http_fallback() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .add_interceptors(vec![Arc::new(RejectSockJsRequest)])
            .with_sock_js()
            .set_websocket_enabled(false);

        let sockjs_services = registry.sockjs_services(&[]);
        let service = sockjs_services[0].1.clone();
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/events/000/rejected-session/xhr")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(service.get_session("rejected-session").is_none());
    }

    #[tokio::test]
    async fn native_sockjs_websocket_runs_application_message_roundtrip() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_heartbeat_time(30);
        let sockjs_services = registry.sockjs_services(&[]);
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("ws://{address}/events/000/session-ws/websocket");
        let (mut client, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        let open = tokio::time::timeout(std::time::Duration::from_secs(3), client.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(open.into_text().unwrap(), "o");

        let welcome = tokio::time::timeout(std::time::Duration::from_secs(3), client.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(welcome.into_text().unwrap(), "a[\"welcome\"]");

        let heartbeat = tokio::time::timeout(std::time::Duration::from_secs(2), client.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(heartbeat.into_text().unwrap(), "h");

        client
            .send(tokio_tungstenite::tungstenite::Message::Text(
                r#"["hello"]"#.into(),
            ))
            .await
            .unwrap();
        let echo = tokio::time::timeout(std::time::Duration::from_secs(3), client.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(echo.into_text().unwrap(), "a[\"echo: hello\"]");

        client.close(None).await.unwrap();
        server.abort();
    }

    /// Reports the handshake attributes it was given, so a test can see them.
    struct AttributeEcho;

    #[async_trait]
    impl WebSocketHandler for AttributeEcho {
        async fn on_open(
            &self,
            session: &crate::server::support::ws_session::WebSocketSession,
        ) -> WSResult<()> {
            let value = session
                .attribute("principal")
                .map(ToString::to_string)
                .unwrap_or_else(|| "<missing>".to_owned());
            // The payload only: the SockJS layer wraps it into an `a[...]` frame.
            session
                .send_message(Message::Text(format!("principal: {value}").into()))
                .await
                .map_err(|error| format!("failed to report the attribute: {error:?}"))?;
            Ok(())
        }

        async fn on_message(
            &self,
            _session: &crate::server::support::ws_session::WebSocketSession,
            _message: Message,
        ) -> WSResult<()> {
            Ok(())
        }

        async fn on_error(
            &self,
            _session: &crate::server::support::ws_session::WebSocketSession,
            _error: Box<dyn Error + Send + Sync>,
        ) -> WSResult<()> {
            Ok(())
        }

        async fn on_close(
            &self,
            _session: &crate::server::support::ws_session::WebSocketSession,
            _close: Option<CloseFrame>,
        ) -> WSResult<()> {
            Ok(())
        }
    }

    struct AttributeSetter;

    #[async_trait]
    impl HandshakeInterceptor for AttributeSetter {
        async fn before_handshake(
            &self,
            _request: &mut dyn next_web_core::http::HttpRequest,
            _response: &mut dyn HttpResponse,
            attributes: &mut HashMap<String, AnyValue>,
        ) -> Result<bool, BoxError> {
            attributes.insert("principal".to_owned(), AnyValue::from("alice"));
            Ok(true)
        }
    }

    #[tokio::test]
    async fn sockjs_hands_the_handshake_attributes_to_the_handler() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(AttributeEcho), vec!["/events".to_string()])
            .with_sock_js()
            .set_heartbeat_time(30);
        let interceptors: Vec<Arc<dyn HandshakeInterceptor>> = vec![Arc::new(AttributeSetter)];
        let sockjs_services = registry.sockjs_services(&interceptors);
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("ws://{address}/events/000/session-attributes/websocket");
        let (mut client, _) = tokio_tungstenite::connect_async(url).await.unwrap();

        // Read past the open frame and any heartbeat that arrives first.
        let mut reported = None;
        for _ in 0..12 {
            let frame = tokio::time::timeout(std::time::Duration::from_secs(3), client.next())
                .await
                .expect("a frame arrives")
                .expect("the stream is open")
                .expect("the frame is not an error");
            let Ok(text) = frame.into_text() else {
                continue;
            };
            if text.starts_with("a[") {
                reported = Some(text);
                break;
            }
        }

        // Before the attributes were threaded through the SockJS service, the
        // map the interceptor filled was dropped when the request ended and the
        // handler saw nothing.
        assert_eq!(reported.as_deref(), Some("a[\"principal: alice\"]"));

        client.close(None).await.unwrap();
        server.abort();
    }

    #[tokio::test]
    async fn sockjs_a_new_receiver_replaces_the_one_that_was_waiting() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_websocket_enabled(false)
            // Far longer than the test waits, so a poll that ends early can only
            // have ended because it was replaced.
            .set_heartbeat_time(10_000);
        let sockjs_services = registry.sockjs_services(&[]);
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let poll = |app: axum::Router| {
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/session-takeover/xhr")
                    .body(Body::empty())
                    .unwrap(),
            )
        };

        // The first poll drains what `on_open` queued, so the next one really has
        // to wait.
        let initial = poll(app.clone()).await.unwrap();
        assert_eq!(initial.status(), StatusCode::OK);
        to_bytes(initial.into_body(), usize::MAX).await.unwrap();

        // Reading the body is what starts the wait and claims the receiver slot.
        let waiting = poll(app.clone()).await.unwrap();
        assert_eq!(waiting.status(), StatusCode::OK);
        let waiting_body =
            tokio::spawn(async move { to_bytes(waiting.into_body(), usize::MAX).await.unwrap() });

        // Let it claim the slot before the replacement arrives.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // A second receiver takes the session over. Its own body has to be driven
        // for that to happen, so it is left waiting and dropped at the end.
        let replacement = poll(app.clone()).await.unwrap();
        assert_eq!(replacement.status(), StatusCode::OK);
        let replacement_body =
            tokio::spawn(async move { to_bytes(replacement.into_body(), usize::MAX).await });

        let replaced = tokio::time::timeout(std::time::Duration::from_secs(1), waiting_body)
            .await
            .expect("the replaced receiver should end as soon as it is taken over")
            .unwrap();

        // And it sends nothing: the client that opened it has already moved on to
        // the replacement, and the replacement will deliver the frames.
        assert!(
            replaced.is_empty(),
            "the replaced receiver sent {replaced:?}"
        );

        replacement_body.abort();
    }

    #[tokio::test]
    async fn sockjs_an_unregistered_transport_creates_no_session() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_websocket_enabled(false);

        let services = registry.sockjs_services(&[]);
        let (_, service) = services.first().expect("a SockJS service was built");

        // `eventsource` is parsed out of the URL but no handler is registered for
        // it. Answering 404 is right; creating the session first and then
        // answering 404 left one behind for the sweep on every such request.
        let response = service
            .handle_request(
                Request::builder()
                    .method("POST")
                    .uri("/events/000/never-created/eventsource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(
            service.get_session("never-created").is_none(),
            "an unregistered transport must not leave a session behind"
        );
    }

    #[tokio::test]
    async fn sockjs_reconnecting_a_session_replaces_the_socket_it_had() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            .with_sock_js()
            .set_heartbeat_time(30);
        let sockjs_services = registry.sockjs_services(&[]);
        let ws_context = WebSocketContext::new(
            WebSocketProperties::default(),
            registry.handler_mapping(),
            Vec::<Arc<dyn HandshakeInterceptor>>::new(),
        );
        let mut apply_router = WebsocketApplyRouter {
            ws_context,
            sockjs_services,
        };
        let app = apply_router.build_router();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("ws://{address}/events/000/session-reconnect/websocket");
        let (mut first, _) = tokio_tungstenite::connect_async(url.clone())
            .await
            .expect("the first connection succeeds");
        let open = tokio::time::timeout(std::time::Duration::from_secs(3), first.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(open.into_text().unwrap(), "o");

        // The same id, as a client that reconnects or switches transport does. The
        // second connection must be usable: it used to be closed without ever
        // sending `o`, because the session counted as already opened.
        let (mut second, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("the reconnect succeeds");

        match tokio::time::timeout(std::time::Duration::from_millis(300), second.next()).await {
            // A heartbeat or a queued frame: it is alive and being served.
            Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_)))) => {
                panic!("the reconnected socket was closed")
            }
            Ok(Some(Ok(_))) | Err(_) => {}
            Ok(Some(Err(error))) => panic!("the reconnected socket errored: {error}"),
            Ok(None) => panic!("the reconnected socket ended"),
        }

        // And the socket it replaced is told, rather than left running with
        // nobody writing to it.
        let closed = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                match first.next().await {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => break,
                    Some(Ok(_)) => continue,
                    Some(Err(_)) => break,
                }
            }
        })
        .await;
        assert!(
            closed.is_ok(),
            "the replaced socket was left open instead of being closed"
        );

        server.abort();
    }

    /// Takes the identity from a header, the way an application's authentication
    /// interceptor would.
    struct IdentityFromHeader;

    #[async_trait]
    impl HandshakeInterceptor for IdentityFromHeader {
        async fn before_handshake(
            &self,
            request: &mut dyn next_web_core::http::HttpRequest,
            _response: &mut dyn HttpResponse,
            attributes: &mut HashMap<String, AnyValue>,
        ) -> Result<bool, BoxError> {
            if let Some(user) = request.header("x-user") {
                attributes.insert(
                    crate::SESSION_IDENTITY_ATTRIBUTE.to_owned(),
                    AnyValue::from(user),
                );
            }
            Ok(true)
        }
    }

    #[tokio::test]
    async fn sockjs_a_session_is_used_only_by_the_identity_that_created_it() {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry
            .add_handler(Arc::new(TestHandler), vec!["/events".to_string()])
            // Before `with_sock_js()`, which copies the interceptors at the moment
            // it is called.
            .add_interceptors(vec![Arc::new(IdentityFromHeader)])
            .with_sock_js()
            .set_websocket_enabled(false)
            .set_heartbeat_time(10_000)
            .set_session_identity_attribute(crate::SESSION_IDENTITY_ATTRIBUTE);
        let sockjs_services = registry.sockjs_services(&[]);
        let (_, service) = sockjs_services.first().expect("a SockJS service was built");

        let poll = |user: Option<&str>| {
            let mut request = Request::builder()
                .method("POST")
                .uri("/events/000/session-identity/xhr");
            if let Some(user) = user {
                request = request.header("x-user", user);
            }
            request.body(Body::empty()).unwrap()
        };

        // The first request creates the session, and therefore owns it.
        let created = service.handle_request(poll(Some("alice"))).await.unwrap();
        assert_eq!(created.status(), StatusCode::OK);

        // A different identity on the same session id is refused, and the session
        // is left alone for its owner rather than handed over.
        let other = service.handle_request(poll(Some("bob"))).await.unwrap();
        assert_eq!(other.status(), StatusCode::NOT_FOUND);
        assert!(service.get_session("session-identity").is_some());

        // The owner still gets in.
        let again = service.handle_request(poll(Some("alice"))).await.unwrap();
        assert_eq!(again.status(), StatusCode::OK);

        // So does a request presenting no identity at all? No: the session was
        // identified, so an unidentified request is somebody else.
        let anonymous = service.handle_request(poll(None)).await.unwrap();
        assert_eq!(anonymous.status(), StatusCode::NOT_FOUND);
    }
}
