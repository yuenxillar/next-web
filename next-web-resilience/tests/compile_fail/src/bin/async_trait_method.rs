//! `#[async_trait]` is applied to the whole impl block, so it expands before the
//! attribute on the method inside it and rewrites the `async fn` into a plain
//! `fn` returning a boxed future. The macro has to refuse, because guarding a
//! future nobody has polled would record an instant success.

use std::fmt;

use next_web_resilience::{
    CallNotPermittedError, async_trait, circuit_breaker, circuitbreaker::CircuitBreaker,
};

#[derive(Debug)]
struct Boom;

impl fmt::Display for Boom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("boom")
    }
}

impl std::error::Error for Boom {}

impl From<CallNotPermittedError> for Boom {
    fn from(_: CallNotPermittedError) -> Self {
        Self
    }
}

struct Gateway {
    breaker: CircuitBreaker,
}

#[async_trait]
trait Charge {
    async fn charge(&self) -> Result<u32, Boom>;
}

#[async_trait]
impl Charge for Gateway {
    #[circuit_breaker(self.breaker)]
    async fn charge(&self) -> Result<u32, Boom> {
        Ok(1)
    }
}

fn main() {}
