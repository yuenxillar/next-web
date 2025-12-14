#[derive(Debug, Clone, serde::Deserialize)]
pub struct CircuitBreakerProperties {
    // The unique identifier of the circuit breaker.
    #[serde(skip_deserializing)]
    pub id: String,

    // Is the circuit breaker enabled.
    pub enabled: bool,

    // Failure  threshold, percentage of triggered circuit breakers.
    #[serde(rename = "failureThreshold")]
    pub failure_threshold: Option<u32>,

    // The time the fuse waits in the OPEN state.
    #[serde(rename = "waitDurationInOpenState")]
    pub wait_duration_in_open_state: Option<u64>,
}

impl Default for CircuitBreakerProperties {
    fn default() -> Self {
        Self {
            id: "TestCircuitBreaker".into(),
            enabled: true,
            failure_threshold: Some(50),
            wait_duration_in_open_state: Some(8),
        }
    }
}
