use next_web_core::async_trait;

/// MQTT消息拦截器接口
/// 用于拦截和处理MQTT消息，可在消息进入系统前进行预处理
/// 
/// MQTT message interceptor interface
/// Used to intercept and process MQTT messages, can pre-process messages before they enter the system
/// 
#[async_trait]
pub trait MessageInterceptor: dyn_clone::DynClone + Send + Sync {

    /// 消息入口拦截方法
    /// 
    /// # 参数 
    /// - topic: 消息主题
    /// - data: 消息原始数据
    /// 
    /// # 返回值
    /// - bool: 是否允许消息继续传递
    /// 
    /// Message entry interception method
    /// 
    /// # Parameters
    /// - topic: Message topic
    /// - data:  Raw message data
    /// 
    /// # Return value
    /// - bool: Whether to allow the message to continue passing
    async fn message_entry(&self, topic: &str, data: &[u8]) -> bool;

    // async fn message_push(&self) -> bool;
}

dyn_clone::clone_trait_object!(MessageInterceptor);