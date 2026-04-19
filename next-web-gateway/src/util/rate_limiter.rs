use std::sync::Arc;

use pingora_limits::rate::Rate;

#[derive(Clone)]
pub struct RateLimiter {
    pub limit: f64,
    pub rate: Arc<Rate>,
}

impl RateLimiter {
    pub fn check_rate(&self, key: &String) -> bool {
        self.rate.observe(key, 1);
        self.rate.rate(key) > self.limit
    }
}
