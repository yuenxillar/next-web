use std::{collections::HashSet, sync::Arc};

use crate::{
    config::{
        sock_js_service_registration::SockJsServiceRegistration,
        ws_handler_registration::WebSocketHandlerRegistration,
    },
    server::{handshake_handler::HandshakeHandler, handshake_interceptor::HandshakeInterceptor},
    ws_handler::WebSocketHandler,
};

/// That gathers all the configuration options
pub struct BaseWebSocketHandlerRegistration {
    handlers: Vec<(HashSet<String>, Arc<dyn WebSocketHandler>)>,
    handshake_handler: Option<Arc<dyn HandshakeHandler>>,
    interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
    allowed_origins: Vec<String>,
    allowed_origin_patterns: Vec<String>,
    sock_js_service_registration: Option<SockJsServiceRegistration>,
}

impl BaseWebSocketHandlerRegistration {
    /// Return the mappings and handlers that have been configured.
    pub fn mappings(
        &mut self,
    ) -> impl IntoIterator<Item = (HashSet<String>, Arc<dyn WebSocketHandler>)> {
        std::mem::take(&mut self.handlers)
    }
}

impl WebSocketHandlerRegistration for BaseWebSocketHandlerRegistration {
    fn add_handler(
        &mut self,
        handler: Arc<dyn WebSocketHandler>,
        paths: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        // if self.handlers.contains_key(&path) {
        //     panic!("Handler already registered for path: {}", path);
        // }

        match self
            .handlers
            .iter_mut()
            .find(|(_, h)| Arc::ptr_eq(h, &handler))
        {
            Some((p, _)) => {
                // Find an existing handler and extend its path
                p.extend(paths.into_iter());
            }
            None => {
                //Not found, insert new handler
                self.handlers
                    .push((HashSet::from_iter(paths.into_iter()), handler));
            }
        }

        self
    }

    fn set_handshake_handler(
        &mut self,
        handshake_handler: Arc<dyn HandshakeHandler>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.handshake_handler = Some(handshake_handler);

        self
    }

    fn add_interceptors(
        &mut self,
        interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.interceptors.extend(interceptors);

        self
    }

    fn set_allowed_origins(
        &mut self,
        origins: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.allowed_origins.clear();
        self.allowed_origins.extend(origins);

        self
    }

    fn set_allowed_origin_patterns(
        &mut self,
        origin_patterns: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.allowed_origin_patterns.clear();
        self.allowed_origin_patterns.extend(origin_patterns);

        self
    }

    fn with_sock_js<'a>(&'a mut self) {
        // self.sock_js_service_registration = Some(SockJsServiceRegistration::default());

        // if !self.interceptors.is_empty() {
        //     self.sock_js_service_registration
        //         .as_mut()
        //         .map(|s| s.set_interceptors(self.interceptors.clone()));
        // }

        // if let Some(handshake_handler) = self.handshake_handler.as_ref() {
        //     let transport_handler = WebSocketTransportHandler::new(handshake_handler.clone());
        //     self.sock_js_service_registration
        //         .as_mut()
        //         .map(|s| s.set_transport_handler_overrides(transport_handler));
        // }

        // if !self.allowed_origins.is_empty() {
        //     self.sock_js_service_registration
        //         .as_mut()
        //         .map(|s| s.set_allowed_origins(self.allowed_origins.clone()));
        // }

        // if !self.allowed_origin_patterns.is_empty() {
        //     self.sock_js_service_registration
        //         .as_mut()
        //         .map(|s| s.set_allowed_origin_patterns(self.allowed_origin_patterns.clone()));
        // }
    }
}

impl Default for BaseWebSocketHandlerRegistration {
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            handshake_handler: Default::default(),
            interceptors: Default::default(),
            allowed_origins: Default::default(),
            allowed_origin_patterns: Default::default(),
            sock_js_service_registration: Default::default(),
        }
    }
}
