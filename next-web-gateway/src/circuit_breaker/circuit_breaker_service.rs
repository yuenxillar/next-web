//! One circuit breaker instance, created from its configuration.

use crate::config::BreakerConfig;

use super::circuit_breaker_controller::CircuitBreakerController;

/// A breaker plus the id routes use to reference it.
#[derive(Clone)]
pub struct CircuitBreakerService {
    /// Unique identifier of the breaker.
    pub id: String,

    /// The state machine.
    pub controller: CircuitBreakerController,
}

impl CircuitBreakerService {
    /// Build a breaker from its configuration.
    pub fn from_config(id: &str, config: &BreakerConfig) -> Self {
        Self {
            id: id.to_string(),
            controller: CircuitBreakerController::with_permits(
                config.enabled,
                config.failure_threshold,
                config.wait_duration_in_open_state,
                config.half_open_permits,
            ),
        }
    }
}
