//! A rate limiter has to ask a permission store, and that store may live in
//! another process, so the permission is awaited. There is no way to do that from
//! a synchronous function without blocking a thread on a runtime, so the macro
//! refuses.

use next_web_resilience::{rate_limiter, ratelimiter::RateLimiter};

struct Api {
    limiter: RateLimiter,
}

impl Api {
    #[rate_limiter(self.limiter)]
    fn login(&self) -> Result<u32, std::io::Error> {
        Ok(1)
    }
}

fn main() {}
