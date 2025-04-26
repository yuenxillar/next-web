use rudi_dev::Singleton;
use std::sync::Arc;

use crate::properties::ws_properties::WebSocketProperties;

use super::handler::WebSocketHandler;

/// WebSocket上下文，提供WebSocket相关配置和处理器

#[derive(Clone)]
pub struct WebSocketContext {
    properties: WebSocketProperties,
    handlers: matchit::Router<Arc<dyn WebSocketHandler>>,
}

impl WebSocketContext {
    ///
    pub fn properties(&self) -> &WebSocketProperties {
        &self.properties
    }

    /// 添加WebSocket处理器
    pub fn add_handler(&mut self, path: &str, handler: Arc<dyn WebSocketHandler>) {
        let _ = self.handlers.insert(path, handler);
    }
    
    /// 根据路径获取处理器
    pub fn get_handler(&self, path: &str) -> Option<&Arc<dyn WebSocketHandler>> {
        match self.handlers.at(path) {
            Ok(matched) => Some(matched.value),
            Err(_) => None
        }
    }
}

#[Singleton]
impl WebSocketContext {
    #[autowired]
    fn private_constructor(#[autowired] properties: WebSocketProperties) -> Self {
        Self {
            properties,
            handlers: matchit::Router::new(),
        }
    }
}
