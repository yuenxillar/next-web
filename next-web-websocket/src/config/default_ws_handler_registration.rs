use std::ops::{Deref, DerefMut};

use crate::{
    config::{
        base_ws_handler_registration::BaseWebSocketHandlerRegistration,
        ws_handler_registration::WebSocketHandlerRegistration,
    },
    server::{handshake_handler::HandshakeHandler, handshake_interceptor::HandshakeInterceptor},
    ws_handler::WebSocketHandler,
};

pub struct DefaultWebSocketHandlerRegistration {
    base: BaseWebSocketHandlerRegistration,
}

impl WebSocketHandlerRegistration for DefaultWebSocketHandlerRegistration {
    fn add_handler(
        &mut self,
        handler: std::sync::Arc<dyn WebSocketHandler>,
        paths: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.base.add_handler(handler, paths)
    }

    fn set_handshake_handler(
        &mut self,
        handshake_handler: std::sync::Arc<dyn HandshakeHandler>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.base.set_handshake_handler(handshake_handler)
    }

    fn add_interceptors(
        &mut self,
        interceptors: Vec<std::sync::Arc<dyn HandshakeInterceptor>>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.base.add_interceptors(interceptors)
    }

    fn set_allowed_origins(
        &mut self,
        origins: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.base.set_allowed_origins(origins)
    }

    fn set_allowed_origin_patterns(
        &mut self,
        origin_patterns: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.base.set_allowed_origin_patterns(origin_patterns)
    }

    fn with_sock_js(&mut self) {
        self.base.with_sock_js()
    }
}

impl Default for DefaultWebSocketHandlerRegistration {
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}

impl Deref for DefaultWebSocketHandlerRegistration {
    type Target = BaseWebSocketHandlerRegistration;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DefaultWebSocketHandlerRegistration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
