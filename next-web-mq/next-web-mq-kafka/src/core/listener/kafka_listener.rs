use next_web_core::{async_trait, error::BoxError};

use crate::core::record::ConsumerRecord;

/// Message listener contract for Kafka consumers.
#[async_trait]
pub trait KafkaListener: Send + Sync {
    async fn on_message(&self, record: ConsumerRecord) -> Result<(), BoxError>;
}
