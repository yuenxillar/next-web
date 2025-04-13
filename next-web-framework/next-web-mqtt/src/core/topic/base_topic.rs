use next_web_core::async_trait;

#[async_trait]
pub trait BaseTopic: Send + Sync {

    /// 
    fn topic(&self) -> &'static str;

    /// 
    async fn consume(&mut self, topic: &str, message: &[u8]) {
        println!("接受到消息， 消息 Topic: {}, 数据内容: {:?}", topic,  String::from_utf8_lossy(message));
    }
}