use std::sync::Arc;

use pingora_limits::rate::Rate;

pub static RATE_KEY: &str = "req";

#[derive(Clone)]
pub struct RateLimiter {
    pub limit: f64,
    pub rate: Arc<Rate>,
}

impl RateLimiter {
    pub fn check_rate(&self) -> bool {
        self.rate.rate(&RATE_KEY) >= self.limit
    }
}
