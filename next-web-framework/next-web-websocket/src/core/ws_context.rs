use rudi_dev::Singleton;
use std::{panic, sync::Arc};

use crate::properties::ws_properties::WebSocketProperties;

use super::handler::WebSocketHandler;

/// WebSocket 上下文，提供 WebSocket 相关配置和处理器。
///
/// 该结构用于管理 WebSocket 的全局配置（如超时、最大连接数等）以及路径与处理器之间的映射关系。
/// 支持添加处理器、根据路径查找处理器等操作。
///
/// WebSocket context that provides WebSocket-related configuration and handlers.
///
/// This structure is used to manage global WebSocket configurations (e.g., timeouts, max connections)
/// and the mapping between paths and their corresponding WebSocket handlers.
/// It supports adding handlers and retrieving them based on URL paths.
///
#[derive(Clone)]
pub struct WebSocketContext {
    properties: WebSocketProperties,
    handlers: matchit::Router<Arc<dyn WebSocketHandler>>,
}

impl WebSocketContext {
    ///
    /// 获取当前 WebSocket 上下文的配置属性。
    ///
    /// 返回对内部 `WebSocketProperties` 实例的只读引用。
    ///
    /// Gets the configuration properties of the current WebSocket context.
    ///
    /// Returns an immutable reference to the internal `WebSocketProperties` instance.
    ///
    pub fn properties(&self) -> &WebSocketProperties {
        &self.properties
    }

    ///
    /// 添加一个 WebSocket 处理器到指定路径。
    ///
    /// # 参数
    /// - `path`: WebSocket 路径（例如：`/ws/chat`）
    /// - `handler`: 实现了 `WebSocketHandler` trait 的处理器实例，需使用 `Arc` 包裹以支持多线程共享。
    ///
    /// Adds a WebSocket handler for the specified path.
    ///
    /// # Arguments
    /// - `path`: The WebSocket path (`/ws/chat`)
    /// - `handler`: A handler implementing the `WebSocketHandler` trait, wrapped in `Arc` for thread-safe sharing.
    ///
    pub fn add_handler(&mut self, path: &str, handler: Arc<dyn WebSocketHandler>) {
        self.handlers
            .insert(path, handler)
            .unwrap_or_else(|e| panic!("Failed to add handler: {:?}", e));
    }

    ///
    /// 根据路径获取对应的 WebSocket 处理器。
    ///
    /// # 参数
    /// - `path`: 请求的 WebSocket 路径。
    ///
    /// # 返回值
    /// - 如果找到匹配的处理器，返回其 `Arc<dyn WebSocketHandler>` 的引用；
    /// - 否则返回 `None`。
    ///
    /// Retrieves the WebSocket handler associated with the given path.
    ///
    /// # Arguments
    /// - `path`: The requested WebSocket path.
    ///
    /// # Returns
    /// - `Some(&Arc<dyn WebSocketHandler>)` if a matching handler is found;
    /// - `None` otherwise.
    ///
    pub fn get_handler(&self, path: &str) -> Option<&Arc<dyn WebSocketHandler>> {
        match self.handlers.at(path) {
            Ok(matched) => Some(matched.value),
            Err(_) => None,
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
