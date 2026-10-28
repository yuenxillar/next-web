//! Without a `fallback`, a rejection is converted into the function's own error
//! with `Into::into`, so that error needs a `From` implementation for the
//! rejection type. This one has none, and the diagnostic comes from the compiler
//! rather than from the macro — which is the point of the case: the requirement
//! is real and shows up where a reader can act on it.

use std::fmt;

use next_web_resilience::{circuit_breaker, circuitbreaker::CircuitBreaker};

#[derive(Debug)]
struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("my error")
    }
}

impl std::error::Error for MyError {}

struct Api {
    breaker: CircuitBreaker,
}

impl Api {
    #[circuit_breaker(self.breaker)]
    async fn charge(&self) -> Result<u32, MyError> {
        Ok(1)
    }
}

fn main() {}
