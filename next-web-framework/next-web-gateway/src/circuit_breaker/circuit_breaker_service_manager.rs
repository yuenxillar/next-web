use hashbrown::HashMap;

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
        for (service, provider) in self.services.iter().zip(fallback_providers.iter()) {
            if service.0.eq(provider.id()) {
                let var = &service.1.controller;
                var.set_on_open(move || println!("打开了熔断器，请稍后再试！！"))
                    .await;
                var.set_on_half_open(move || println!("打开了半开熔断器，请稍后再试！！"))
                    .await;
            }
        }
    }
}
