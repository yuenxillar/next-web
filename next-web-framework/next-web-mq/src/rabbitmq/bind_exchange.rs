#[derive(Debug, Clone)]
pub struct BindExchange {
    queue_name: String,
    exchange_name: String,
    routing_key: String,
}

impl BindExchange {
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

pub trait BindExchangeBuilder {
    fn value(&self) -> Vec<BindExchange>;
}
