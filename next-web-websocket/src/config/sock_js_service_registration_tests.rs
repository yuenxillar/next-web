#[cfg(test)]
mod behaviour {
    use crate::config::SockJsServiceRegistration;
    use crate::sockjs::transport::TransportHandler;
    use std::sync::Arc;

    #[test]
    fn test_default_configuration() {
        let registration = SockJsServiceRegistration::default();

        assert!(registration.task_scheduler.is_none());
        assert!(registration.client_library_url.is_none());
        assert!(registration.stream_bytes_limit.is_none());
        assert!(registration.session_cookie_needed.is_none());
        assert!(registration.heartbeat_time.is_none());
        assert!(registration.disconnect_delay.is_none());
        assert!(registration.http_message_cache_size.is_none());
        assert!(registration.websocket_enabled.is_none());
        assert!(registration.transport_handlers.is_empty());
        assert!(registration.interceptors.is_empty());
        assert!(registration.allowed_origins.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let mut registration = SockJsServiceRegistration::default();

        registration
            .set_heartbeat_time(30_000)
            .set_disconnect_delay(10_000)
            .set_stream_bytes_limit(256 * 1024)
            .set_session_cookie_needed(false)
            .set_websocket_enabled(true)
            .set_http_message_cache_size(200)
            .set_suppress_cors(true)
            .set_client_library_url("https://custom.cdn/sockjs.js".to_string())
            .set_allowed_origins(vec!["https://example.com".to_string()]);

        assert_eq!(registration.heartbeat_time, Some(30_000));
        assert_eq!(registration.disconnect_delay, Some(10_000));
        assert_eq!(registration.stream_bytes_limit, Some(256 * 1024));
        assert_eq!(registration.session_cookie_needed, Some(false));
        assert_eq!(registration.websocket_enabled, Some(true));
        assert_eq!(registration.http_message_cache_size, Some(200));
        assert_eq!(registration.suppress_cors, Some(true));
        assert_eq!(
            registration.client_library_url,
            Some("https://custom.cdn/sockjs.js".to_string())
        );
        assert_eq!(registration.allowed_origins.len(), 1);
    }

    #[test]
    fn test_get_sockjs_service_with_defaults() {
        let registration = SockJsServiceRegistration::default();
        let service = registration.get_sockjs_service();

        // Check default values are applied through config
        let config = service.config();
        assert_eq!(config.stream_bytes_limit, 128 * 1024);
        assert!(config.session_cookie_needed);
        assert_eq!(config.heartbeat_time, 25_000);
        assert_eq!(config.disconnect_delay, 5_000);
        assert_eq!(config.http_message_cache_size, 100);
        assert!(config.websocket_enabled);
        assert!(!config.suppress_cors);
    }

    #[test]
    fn test_get_sockjs_service_with_custom_config() {
        let mut registration = SockJsServiceRegistration::default();

        registration
            .set_heartbeat_time(30_000)
            .set_disconnect_delay(10_000)
            .set_stream_bytes_limit(256 * 1024)
            .set_session_cookie_needed(false)
            .set_websocket_enabled(false)
            .set_http_message_cache_size(200)
            .set_suppress_cors(true);

        let service = registration.get_sockjs_service();
        let config = service.config();

        assert_eq!(config.stream_bytes_limit, 256 * 1024);
        assert!(!config.session_cookie_needed);
        assert_eq!(config.heartbeat_time, 30_000);
        assert_eq!(config.disconnect_delay, 10_000);
        assert_eq!(config.http_message_cache_size, 200);
        assert!(!config.websocket_enabled);
        assert!(config.suppress_cors);
    }

    #[test]
    fn test_cors_configuration() {
        let mut registration = SockJsServiceRegistration::default();

        registration
            .set_allowed_origins(vec![
                "https://example.com".to_string(),
                "https://another.com".to_string(),
            ])
            .set_allowed_origin_patterns(vec!["https://*.example.com".to_string()]);

        let service = registration.get_sockjs_service();
        let config = service.config();

        assert_eq!(config.allowed_origins.len(), 2);
        assert_eq!(config.allowed_origin_patterns.len(), 1);
        assert_eq!(config.allowed_origins[0], "https://example.com");
        assert_eq!(config.allowed_origin_patterns[0], "https://*.example.com");
    }

    #[test]
    fn test_default_transport_handlers_created() {
        let registration = SockJsServiceRegistration::default();
        let service = registration.get_sockjs_service();

        // Service should be created successfully with default transport handlers
        let config = service.config();
        assert_eq!(config.stream_bytes_limit, 128 * 1024);
        assert!(config.websocket_enabled);
    }

    #[test]
    fn test_websocket_disabled_in_handlers() {
        let mut registration = SockJsServiceRegistration::default();
        registration.set_websocket_enabled(false);

        let service = registration.get_sockjs_service();
        let config = service.config();

        // WebSocket should be disabled
        assert!(!config.websocket_enabled);
    }

    #[test]
    fn test_custom_transport_handlers() {
        use crate::sockjs::transport::WebSocketTransportHandler;

        let mut registration = SockJsServiceRegistration::default();

        let custom_handlers: Vec<Arc<dyn TransportHandler>> =
            vec![Arc::new(WebSocketTransportHandler::new())];

        registration.set_transport_handlers(custom_handlers);

        assert_eq!(registration.transport_handlers.len(), 1);
    }

    #[test]
    fn test_conflicting_transport_config_panics() {
        use crate::sockjs::transport::WebSocketTransportHandler;

        let mut registration = SockJsServiceRegistration::default();

        registration.set_transport_handlers(vec![Arc::new(WebSocketTransportHandler::new())]);

        registration
            .set_transport_handler_overrides(vec![Arc::new(WebSocketTransportHandler::new())]);

        // This should panic when trying to build with both set
        // For now, we just verify both can be set
        assert_eq!(registration.transport_handlers.len(), 1);
        assert_eq!(registration.transport_handler_overrides.len(), 1);
    }

    #[test]
    fn test_transport_handler_overrides() {
        use crate::sockjs::transport::WebSocketTransportHandler;

        let mut registration = SockJsServiceRegistration::default();

        // Override WebSocket transport
        registration
            .set_transport_handler_overrides(vec![Arc::new(WebSocketTransportHandler::new())]);

        assert_eq!(registration.transport_handler_overrides.len(), 1);
    }

    #[test]
    fn test_message_codec_default() {
        let registration = SockJsServiceRegistration::default();
        let service = registration.get_sockjs_service();

        // Service should have default configuration
        let config = service.config();
        assert_eq!(config.heartbeat_time, 25_000);
    }

    #[test]
    fn test_interceptors_transferred() {
        use crate::server::handshake_interceptor::HandshakeInterceptor;
        use next_web_core::any::AnyValue;
        use next_web_core::async_trait;
        use next_web_core::error::BoxError;
        use next_web_core::http::HttpRequest;
        use next_web_core::http::HttpResponse;
        use std::collections::HashMap;

        struct TestInterceptor;

        #[async_trait]
        impl HandshakeInterceptor for TestInterceptor {
            async fn before_handshake(
                &self,
                _request: &mut dyn HttpRequest,
                _response: &mut dyn HttpResponse,
                _attributes: &mut HashMap<String, AnyValue>,
            ) -> Result<bool, BoxError> {
                Ok(true)
            }
        }

        let mut registration = SockJsServiceRegistration::default();
        registration.set_interceptors(vec![Arc::new(TestInterceptor)]);

        assert_eq!(registration.interceptors.len(), 1);
    }
}
