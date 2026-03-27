#[derive(Debug, Clone)]
pub struct BindExchange {
    queue_name: String,
    exchange_name: String,
    routing_key: String,
}

impl BindExchange {
    pub fn new(
        queue_name: impl Into<String>,
        exchange_name: impl Into<String>,
        routing_key: impl Into<String>,
    ) -> Self {
        Self {
            queue_name: queue_name.into(),
            exchange_name: exchange_name.into(),
            routing_key: routing_key.into(),
        }
    }

    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    pub fn exchange_name(&self) -> &str {
        &self.exchange_name
    }

    pub fn routing_key(&self) -> &str {
        &self.routing_key
    }
}

pub trait BindExchangeBuilder: Send + Sync {
    fn value(&self) -> Vec<BindExchange>;
}
