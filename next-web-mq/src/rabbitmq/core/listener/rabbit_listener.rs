use amqprs::channel::ConsumerMessage;
use next_web_core::async_trait;

#[async_trait]
pub trait  RabbitListener: Send + Sync{

    fn queue(&self) -> String;

    fn consumer_tag(&self) -> String {
        String::new()
    }

    async fn on_message(&mut self, msg: ConsumerMessage);
}