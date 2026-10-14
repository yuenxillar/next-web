use std::{collections::HashMap, sync::Arc};

use crate::sockjs::{
    service::TransportHandlingSockJsService,
    transport::{
        SockJsServiceConfig, TransportHandler, TransportType, WebSocketTransportHandler,
        XhrPollingTransportHandler, XhrStreamingTransportHandler,
    },
};

/// Default SockJS service with standard transport handlers.
///
/// This provides a convenient way to create a SockJS service with
/// the default set of transport handlers (WebSocket, XHR streaming,
/// XHR polling).
pub struct DefaultSockJsService;

impl DefaultSockJsService {
    /// Creates a new default SockJS service with standard transports.
    ///
    /// # Arguments
    ///
    /// * `config` - Service configuration
    ///
    /// # Returns
    ///
    /// A configured `TransportHandlingSockJsService` with default handlers.
    pub fn new(config: SockJsServiceConfig) -> TransportHandlingSockJsService {
        let mut transport_handlers: HashMap<TransportType, Arc<dyn TransportHandler>> =
            HashMap::new();

        // Register default transport handlers
        if config.websocket_enabled {
            transport_handlers.insert(
                TransportType::WebSocket,
                Arc::new(WebSocketTransportHandler::new()),
            );
        }

        transport_handlers.insert(
            TransportType::XhrStreaming,
            Arc::new(XhrStreamingTransportHandler::new()),
        );

        transport_handlers.insert(
            TransportType::XhrPolling,
            Arc::new(XhrPollingTransportHandler::new()),
        );

        TransportHandlingSockJsService::new(config, transport_handlers)
    }

    /// Creates a new service with custom transport handler overrides.
    ///
    /// This allows replacing specific transport handlers while keeping
    /// the defaults for others.
    ///
    /// # Arguments
    ///
    /// * `config` - Service configuration
    /// * `overrides` - Custom transport handlers to override defaults
    pub fn with_overrides(
        config: SockJsServiceConfig,
        overrides: HashMap<TransportType, Arc<dyn TransportHandler>>,
    ) -> TransportHandlingSockJsService {
        let mut transport_handlers: HashMap<TransportType, Arc<dyn TransportHandler>> =
            HashMap::new();

        // Start with defaults
        if config.websocket_enabled && !overrides.contains_key(&TransportType::WebSocket) {
            transport_handlers.insert(
                TransportType::WebSocket,
                Arc::new(WebSocketTransportHandler::new()),
            );
        }

        if !overrides.contains_key(&TransportType::XhrStreaming) {
            transport_handlers.insert(
                TransportType::XhrStreaming,
                Arc::new(XhrStreamingTransportHandler::new()),
            );
        }

        if !overrides.contains_key(&TransportType::XhrPolling) {
            transport_handlers.insert(
                TransportType::XhrPolling,
                Arc::new(XhrPollingTransportHandler::new()),
            );
        }

        // Apply overrides
        for (transport_type, handler) in overrides {
            transport_handlers.insert(transport_type, handler);
        }

        TransportHandlingSockJsService::new(config, transport_handlers)
    }
}
