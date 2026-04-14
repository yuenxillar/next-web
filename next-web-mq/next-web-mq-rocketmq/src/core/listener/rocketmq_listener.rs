use next_web_core::{async_trait, error::BoxError};

use crate::core::message::RocketmqDelivery;

/// Message listener contract for RocketMQ consumers.
#[async_trait]
pub trait RocketmqListener: Send + Sync {
    /// Handles a message delivered from a RocketMQ topic subscription.
    async fn on_message(&self, delivery: RocketmqDelivery) -> Result<(), BoxError>;
}
