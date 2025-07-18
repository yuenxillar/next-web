use super::{application_listener::ApplicationListener, key::Key};

/// 应用事件多播器
/// Application event multicaster
pub trait ApplicationEventMulticaster: Send + Sync {
    /// 添加应用事件监听器
    /// Add application event listener
    fn add_application_listener(&mut self, listener: Box<dyn ApplicationListener>);

    /// 移除应用事件监听器
    /// Remove application event listener
    fn remove_application_listener(&mut self, key: &Key);
}
