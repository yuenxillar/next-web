use crate::properties::circuit_breaker_properties::CircuitBreakerProperties;

use super::circuit_breaker_controller::CircuitBreakerController;

#[derive(Clone)]
pub struct CircuitBreakerService {
    // The unique identifier of the circuit breaker.
    pub id: String,

    // Is the circuit breaker active or not.
    pub active: bool,

    // The controller of the circuit breaker.
    pub controller: CircuitBreakerController,
}

impl Into<CircuitBreakerService> for CircuitBreakerProperties {
    fn into(self) -> CircuitBreakerService {
        CircuitBreakerService {
            id: self.id,
            active: self.enabled,
            controller: CircuitBreakerController::new(
                self.failure_threshold.unwrap_or(10),
                std::time::Duration::from_secs(self.wait_duration_in_open_state.unwrap_or(5)),
            ),
        }
    }
}
