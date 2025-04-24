use std::sync::Arc;

use super::handler::WebSocketHandler;

/// 
pub trait WebSocketHandlerRegistry: Send + Sync {

    /// 
    fn handlers(&self) -> Vec<HandlerRegistry>;
}


/// 
/// 
#[derive(Clone)]
pub struct HandlerRegistry {
    /// paths
    pub paths: Vec<&'static str>, 
    /// handler
    pub handler: Arc<dyn WebSocketHandler>
}