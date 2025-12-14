use next_web_core::DynClone;
use next_web_core::async_trait;

/// Redis键过期事件监听 trait
///  Redis expired keys event listener trait
/// 注意：需要Redis服务器配置 `notify-keyspace-events Ex` 才能生效
/// NOTE: Requires Redis server configuration `notify-keyspace-events Ex` to work
#[async_trait]
pub trait RedisExpiredKeysEvent: DynClone + Send + Sync {
    /// 处理过期键事件的回调方法 / Callback for handling expired key events
    ///
    /// 参数 / Parameters:
    /// - `message`: 原始消息字节（通常是过期键名） / Raw message bytes (usually expired key name)
    /// - `pattern`: 匹配的事件模式（如 "__keyevent@*__:expired"） / Event pattern matched (e.g. "__keyevent@*__:expired")
    async fn on_message(&mut self, message: &[u8], pattern: &[u8]);
}

// 实现 trait object 的克隆支持
// Implements cloning support for trait objects
next_web_core::clone_trait_object!(RedisExpiredKeysEvent);
