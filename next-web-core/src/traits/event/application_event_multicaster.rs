use std::time::Duration;

use async_trait::async_trait;

use super::application_listener::ApplicationListener;
use crate::traits::event::application_event::ApplicationEvent;

/// 应用事件多播器
///
/// Application event multicaster
#[async_trait]
pub trait ApplicationEventMulticaster
where
    Self: Send + Sync,
{
    /// 添加应用事件监听器
    ///
    /// Add application event listener
    async fn add_application_listener<L, E>(&mut self, id: String, listener: L)
    where
        L: ApplicationListener<E>,
        E: ApplicationEvent;

    /// 移除应用事件监听器
    ///
    /// Remove application event listener
    async fn remove_application_listener<E>(&mut self, id: String)
    where
        E: ApplicationEvent;

    /// 移除所有应用事件监听器
    ///
    /// Remove all application event listeners
    async fn remove_all_listeners(&mut self);

    /// 广播应用事件
    ///
    /// Multicast application event
    async fn multicast_event<E>(&self, event: E) -> Result<(), MulticastError>
    where
        E: ApplicationEvent;
}

/// Errors that can occur during event multicasting
///
/// 事件广播过程中可能发生的错误
#[derive(Debug, thiserror::Error)]
pub enum MulticastError {
    /// No listeners registered for this event type
    ///
    /// 没有为该事件类型注册监听器
    #[error("No listeners registered for event type: {0}")]
    NoListeners(String),

    /// Failed to send event to processor channel
    ///
    /// 发送事件到处理器通道失败
    #[error("Failed to send event: {0}")]
    SendError(String),

    /// Send operation timed out
    ///
    /// 发送操作超时
    #[error("Send operation timed out after {0:?}")]
    Timeout(Duration),

    /// Event processor channel is closed
    ///
    /// 事件处理器通道已关闭
    #[error("Event processor channel is closed")]
    ChannelClosed,

    /// Failed to downcast event to expected type
    ///
    /// 无法将事件转换为期望的类型
    #[error("Failed to downcast event")]
    DowncastError,

    /// Listener execution failed
    ///
    /// 监听器执行失败
    #[error("Listener execution failed: {0}")]
    ListenerError(String),

    /// Other errors
    ///
    /// 其他错误
    #[error("Multicast error: {0}")]
    Other(String),
}
