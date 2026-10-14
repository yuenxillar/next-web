//! Guarding a method that implements a trait.
//!
//! Trait methods are often written with `#[async_trait]`, which rewrites each
//! `async fn` into a plain `fn` returning a boxed future. That attribute sits on
//! the whole `impl` block and therefore expands *before* any attribute on the
//! methods inside it, so by the time a resilience macro runs there is no
//! `async fn` left to guard. The macro refuses rather than generating code that
//! would record an instant success for a future nobody has polled, and its
//! message names the cause — the `async_trait` case in the compile-fail suite
//! pins that message down.
//!
//! There are two ways out, and the one below needs no extra machinery: an
//! `async fn` in a trait is native, so as long as the trait does not have to be
//! used as a `dyn Trait`, `#[async_trait]` can simply be dropped. The other is to
//! leave the attribute off the method and drive the pattern from inside the body.

use std::{fmt, sync::Arc};

use next_web_resilience::{
    CallNotPermittedError, circuit_breaker,
    circuitbreaker::{CircuitBreaker, CircuitBreakerConfig},
};

/// The error the guarded method returns.
#[derive(Debug)]
struct Boom;

impl fmt::Display for Boom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the gateway refused the call")
    }
}

impl std::error::Error for Boom {}

impl From<CallNotPermittedError> for Boom {
    fn from(_: CallNotPermittedError) -> Self {
        Self
    }
}

/// A native trait: the method is still an `async fn` when the macro expands.
///
/// `async_fn_in_trait` only warns because a `dyn Trait` cannot name the auto
/// trait bounds of the returned future; this trait is only ever used statically.
#[allow(async_fn_in_trait)]
trait Charge {
    /// Charges the gateway.
    async fn charge(&self) -> Result<u32, Boom>;
}

/// A gateway whose calls are guarded by a circuit breaker.
struct Gateway {
    /// The breaker the macro resolves through `self.breaker`.
    breaker: Arc<CircuitBreaker>,
}

impl Charge for Gateway {
    #[circuit_breaker(self.breaker)]
    async fn charge(&self) -> Result<u32, Boom> {
        Ok(3)
    }
}

#[tokio::test]
async fn a_native_trait_method_can_be_guarded() {
    let gateway = Gateway {
        breaker: Arc::new(CircuitBreaker::new(
            "gateway",
            CircuitBreakerConfig::builder()
                .sliding_window_size(10)
                .minimum_number_of_calls(100)
                .build(),
        )),
    };

    assert_eq!(gateway.charge().await.expect("the breaker is closed"), 3);
    assert_eq!(gateway.breaker.snapshot().number_of_successful_calls, 1);
    assert_eq!(gateway.breaker.snapshot().number_of_buffered_calls, 1);
}
