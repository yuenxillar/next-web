use async_trait::async_trait;

use crate::common::key::Key;

use super::application_listener::ApplicationListener;

/// 应用事件多播器
///
/// Application event multicaster
#[async_trait]
pub trait ApplicationEventMulticaster: Send + Sync {
    /// 添加应用事件监听器
    ///
    /// Add application event listener
    async fn add_application_listener(&mut self, listener: Box<dyn ApplicationListener>);

    /// 移除应用事件监听器
    ///
    /// Remove application event listener
    async fn remove_application_listener(&mut self, key: &Key);
}
