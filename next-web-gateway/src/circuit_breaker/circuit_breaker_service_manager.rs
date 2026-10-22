//! The circuit breaker registry, keyed by breaker id.

use hashbrown::HashMap;
use tracing::info;

use crate::config::BreakerConfig;

use super::{circuit_breaker_service::CircuitBreakerService, fallback_provider::FallbackProvider};

/// Every breaker defined under `next.gateway.breakers`.
#[derive(Clone, Default)]
pub struct CircuitBreakerServiceManager {
    /// Breakers by id; routes reference them with `CircuitBreaker=<id>`.
    pub services: HashMap<String, CircuitBreakerService>,
}

impl CircuitBreakerServiceManager {
    /// Build the registry from the validated configuration.
    pub fn from_config<T>(breakers: &T) -> Self
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

    /// Whether no breaker is configured.
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Install the state change callbacks of the configured providers.
    pub async fn set_fallback_providers(&mut self, fallback_providers: Vec<Box<dyn FallbackProvider>>) {
        for provider in fallback_providers.iter() {
            if let Some(service) = self.services.get(provider.id()) {
                let controller = &service.controller;
                let id = service.id.clone();
                controller
                    .set_on_open(move || info!(breaker = %id, "circuit breaker opened"))
                    .await;
                let id = service.id.clone();
                controller
                    .set_on_half_open(move || info!(breaker = %id, "circuit breaker half-open"))
                    .await;
            }
        }
    }
}
