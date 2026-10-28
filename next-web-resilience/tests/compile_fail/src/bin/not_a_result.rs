//! A rejection has to reach the caller, so the guarded function must return a
//! `Result`. Returned plainly, without one, there would be nowhere to put it.

use next_web_resilience::{circuit_breaker, circuitbreaker::CircuitBreaker};

struct Api {
    breaker: CircuitBreaker,
}

impl Api {
    #[circuit_breaker(self.breaker)]
    async fn charge(&self) -> u64 {
        1
    }
}

fn main() {}
