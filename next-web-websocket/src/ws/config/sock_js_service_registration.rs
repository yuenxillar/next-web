use std::sync::Arc;

use crate::ws::server::handshake_interceptor::HandshakeInterceptor;

pub struct SockJsServiceRegistration {}
impl SockJsServiceRegistration {
    pub fn set_interceptors(&mut self, interceptors: Vec<Arc<dyn HandshakeInterceptor>>) {
        todo!()
    }
    
    pub fn set_allowed_origin_patterns(&mut self, allowed_origin_patterns: Vec<String>)  {
        todo!()
    }
    
    pub(crate) fn set_allowed_origins(&mut self, allowed_origins: Vec<String>)  {
        todo!()
    }
}

impl Default for SockJsServiceRegistration {
    fn default() -> Self {
        Self {}
    }
}
