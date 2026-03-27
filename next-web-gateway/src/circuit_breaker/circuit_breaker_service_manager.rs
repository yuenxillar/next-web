use hashbrown::HashMap;
use tracing::info;

use super::{circuit_breaker_service::CircuitBreakerService, fallback_provider::FallbackProvider};

#[derive(Clone)]
pub struct CircuitBreakerServiceManager {
    pub services: HashMap<String, CircuitBreakerService>,
}

impl CircuitBreakerServiceManager {
    pub async fn set_fallback_providers(
        &mut self,
        fallback_providers: Vec<Box<dyn FallbackProvider>>,
    ) {
        for provider in fallback_providers.iter() {
            if let Some(service) = self.services.get(provider.id()) {
                let controller = &service.controller;
                controller
                    .set_on_open(move || info!("circuit breaker opened"))
                    .await;
                controller
                    .set_on_half_open(move || info!("circuit breaker half-open"))
                    .await;
            }
        }
    }
}
