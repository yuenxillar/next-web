use amqprs::channel::ConsumerMessage;
use next_web_core::{async_trait, error::BoxError};

/// Message listener contract for RabbitMQ consumers.
#[async_trait]
pub trait RabbitListener: Send + Sync {
    async fn on_message(&self, message: ConsumerMessage) -> Result<(), BoxError>;
}
