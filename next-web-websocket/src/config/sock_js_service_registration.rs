use std::sync::Arc;

use crate::server::handshake_interceptor::HandshakeInterceptor;

/// Stores SockJS-specific options in the same spirit as Spring's
/// `SockJsServiceRegistration`.
pub struct SockJsServiceRegistration {
    interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
    allowed_origins: Vec<String>,
    allowed_origin_patterns: Vec<String>,
}

impl SockJsServiceRegistration {
    /// Return the handshake interceptors that will also apply to SockJS
    /// transports.
    pub fn interceptors(&self) -> &[Arc<dyn HandshakeInterceptor>] {
        &self.interceptors
    }

    /// Replace the handshake interceptors configured for SockJS requests.
    pub fn set_interceptors(&mut self, interceptors: Vec<Arc<dyn HandshakeInterceptor>>) {
        self.interceptors.clear();
        self.interceptors.extend(interceptors);
    }

    /// Return the explicitly allowed origins for SockJS requests.
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    /// Replace the allowed origin patterns for SockJS requests.
    pub fn set_allowed_origin_patterns(&mut self, allowed_origin_patterns: Vec<String>) {
        self.allowed_origin_patterns.clear();
        self.allowed_origin_patterns.extend(allowed_origin_patterns);
    }

    /// Return the allowed origin patterns for SockJS requests.
    pub fn allowed_origin_patterns(&self) -> &[String] {
        &self.allowed_origin_patterns
    }

    pub(crate) fn set_allowed_origins(&mut self, allowed_origins: Vec<String>) {
        self.allowed_origins.clear();
        self.allowed_origins.extend(allowed_origins);
    }
}

impl Default for SockJsServiceRegistration {
    fn default() -> Self {
        Self {
            interceptors: Vec::new(),
            allowed_origins: Vec::new(),
            allowed_origin_patterns: Vec::new(),
        }
    }
}
