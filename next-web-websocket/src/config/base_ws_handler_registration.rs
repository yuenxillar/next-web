use std::{collections::HashSet, sync::Arc};

use crate::{
    config::{
        sock_js_service_registration::SockJsServiceRegistration,
        ws_handler_registration::WebSocketHandlerRegistration,
    },
    server::handshake_interceptor::HandshakeInterceptor,
    ws_handler::WebSocketHandler,
};

/// That gathers all the configuration options
pub struct BaseWebSocketHandlerRegistration {
    handlers: Vec<(HashSet<String>, Arc<dyn WebSocketHandler>)>,
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

    /// Return the SockJS-specific registration if SockJS fallback was enabled.
    pub fn sock_js_service_registration(&self) -> Option<&SockJsServiceRegistration> {
        self.sock_js_service_registration.as_ref()
    }

    /// Return the interceptors configured for this registration.
    pub fn interceptors(&self) -> &[Arc<dyn HandshakeInterceptor>] {
        self.interceptors.as_slice()
    }

    /// Return the explicitly allowed origins configured for this registration.
    pub fn allowed_origins(&self) -> &[String] {
        self.allowed_origins.as_slice()
    }

    /// Return the allowed origin patterns configured for this registration.
    pub fn allowed_origin_patterns(&self) -> &[String] {
        self.allowed_origin_patterns.as_slice()
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

    fn add_interceptors(
        &mut self,
        interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.interceptors.extend(interceptors);
        if let Some(sock_js) = self.sock_js_service_registration.as_mut() {
            sock_js.set_interceptors(self.interceptors.clone());
        }

        self
    }

    fn set_allowed_origins(
        &mut self,
        origins: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.allowed_origins.clear();
        self.allowed_origins.extend(origins);
        if let Some(sock_js) = self.sock_js_service_registration.as_mut() {
            sock_js.set_allowed_origins(self.allowed_origins.clone());
        }

        self
    }

    fn set_allowed_origin_patterns(
        &mut self,
        origin_patterns: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        self.allowed_origin_patterns.clear();
        self.allowed_origin_patterns.extend(origin_patterns);
        if let Some(sock_js) = self.sock_js_service_registration.as_mut() {
            sock_js.set_allowed_origin_patterns(self.allowed_origin_patterns.clone());
        }

        self
    }

    fn with_sock_js(&mut self) -> &mut SockJsServiceRegistration {
        let interceptors = self.interceptors.clone();
        let allowed_origins = self.allowed_origins.clone();
        let allowed_origin_patterns = self.allowed_origin_patterns.clone();

        let sock_js = self
            .sock_js_service_registration
            .get_or_insert_with(SockJsServiceRegistration::default);

        if !interceptors.is_empty() {
            sock_js.set_interceptors(interceptors);
        }
        if !allowed_origins.is_empty() {
            sock_js.set_allowed_origins(allowed_origins);
        }
        if !allowed_origin_patterns.is_empty() {
            sock_js.set_allowed_origin_patterns(allowed_origin_patterns);
        }

        sock_js
    }
}

impl Default for BaseWebSocketHandlerRegistration {
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            interceptors: Default::default(),
            allowed_origins: Default::default(),
            allowed_origin_patterns: Default::default(),
            sock_js_service_registration: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BaseWebSocketHandlerRegistration;
    use crate::config::ws_handler_registration::WebSocketHandlerRegistration;

    #[test]
    fn with_sock_js_copies_existing_origin_configuration() {
        let mut registration = BaseWebSocketHandlerRegistration::default();
        registration.set_allowed_origins(vec!["https://a.example".to_string()]);
        registration.set_allowed_origin_patterns(vec!["https://*.example".to_string()]);

        let sock_js = registration.with_sock_js();

        assert_eq!(sock_js.allowed_origins(), ["https://a.example"]);
        assert_eq!(sock_js.allowed_origin_patterns(), ["https://*.example"]);
    }

    #[test]
    fn setters_keep_sock_js_registration_in_sync() {
        let mut registration = BaseWebSocketHandlerRegistration::default();
        let _ = registration.with_sock_js();

        registration.set_allowed_origins(vec!["https://b.example".to_string()]);
        registration.set_allowed_origin_patterns(vec!["https://*.b.example".to_string()]);

        let sock_js = registration
            .sock_js_service_registration()
            .expect("SockJS registration should exist");

        assert_eq!(sock_js.allowed_origins(), ["https://b.example"]);
        assert_eq!(sock_js.allowed_origin_patterns(), ["https://*.b.example"]);
    }
}
