use std::sync::Arc;

use matchit::Router;

use crate::ws::ws_handler::WebSocketHandler;

#[derive(Clone)]
pub struct WebSocketHandlerMapping {
    original_paths: Vec<String>,

    handlers: Router<Arc<dyn WebSocketHandler>>,
}

impl WebSocketHandlerMapping {
    pub fn new(handlers: Router<Arc<dyn WebSocketHandler>>) -> Self {
        Self {
            handlers,
            original_paths: Default::default(),
        }
    }

    pub fn insert(&mut self, path: &str, handler: Arc<dyn WebSocketHandler>) {
        self.handlers
            .insert(path, handler)
            .and_then(|_| {
                self.original_paths.push(path.to_string());
                Ok(())
            })
            .unwrap_or_else(|e| panic!("Failed to add handler: {:?}", e));
    }

    pub fn get(&self, path: &str) -> Option<&Arc<dyn WebSocketHandler>> {
        match self.handlers.at(path) {
            Ok(matched) => Some(matched.value),
            Err(_) => None,
        }
    }

    pub fn paths(&self) -> &[String] {
        &self.original_paths
    }
}

impl Default for WebSocketHandlerMapping {
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            original_paths: Default::default(),
        }
    }
}
