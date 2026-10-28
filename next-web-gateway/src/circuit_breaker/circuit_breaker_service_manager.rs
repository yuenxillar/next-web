//! The circuit breaker registry, keyed by breaker id.

use hashbrown::HashMap;

use crate::config::BreakerConfig;

use super::circuit_breaker_service::CircuitBreakerService;

/// Every breaker defined under `next.gateway.breakers`.
#[derive(Clone, Default)]
pub(crate) struct CircuitBreakerServiceManager {
    /// Breakers by id; routes reference them with `CircuitBreaker=<id>`.
    pub(crate) services: HashMap<String, CircuitBreakerService>,
}

impl CircuitBreakerServiceManager {
    /// Build the registry from the validated configuration.
    pub(crate) fn from_config<T>(breakers: &T) -> Self
    where
        T: std::borrow::Borrow<std::collections::HashMap<String, BreakerConfig>> + ?Sized,
    {
        let breakers = breakers.borrow();
        let services = breakers
            .iter()
            .map(|(id, config)| (id.clone(), CircuitBreakerService::from_config(id, config)))
            .collect();

        Self { services }
    }
}
