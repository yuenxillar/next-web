
/// 关于MQTT协议的 相关实现
///
/// 其中包含 
/// 1. 连接服务所使用的选项 https://
/// 2. 每个 Topic 的实现消费者
/// 3. 当消息进入 Service 前的拦截处理
///
///
/// # Examples
///
/// ```
/// use rudi::Singleton;
/// use next_web_core::async_trait;
/// 
/// #[SingleOwner(binds = [Self::into_base_topic])]
/// #[derive(Clone)]
/// pub struct TestBaseTopic;
/// 
/// 
/// #[async_trait]
/// impl BaseTopic for TestBaseTopic {
/// 
///     fn topic(&slef) -> &'static str 
///     {
///         "test/#"
///     }
/// 
///     async fn consume(&mut self, topic: &str, message: &[u8]) 
///     {
///         println!("接受到消息， 消息 Topic: {}, 数据内容: {:?}", topic,  String::from_utf8_lossy(message));
///     } 
/// }
/// 
/// impl BaseTopic {
///     
///     fn into_base_topic(self) -> Box<dyn BaseTopic> 
///     {
///         Box::new(self)
///     }
/// }
/// ```
///

pub mod core;
pub mod auto_register;
pub mod properties;
pub mod service;