use thiserror::Error;

/// Errors returned by ID generators.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdGeneratorError {
    /// The node id does not fit in the configured bit width.
    #[error("node id {node_id} exceeds max value {max_node_id}")]
    InvalidNodeId {
        /// The configured node id.
        node_id: u16,
        /// The maximum allowed node id for the snowflake layout.
        max_node_id: u16,
    },
    /// The configured epoch is in the future.
    #[error("epoch {epoch_millis} must not be greater than the current timestamp")]
    EpochInFuture {
        /// The invalid epoch value in milliseconds.
        epoch_millis: u64,
    },
    /// The local clock moved backwards and the generator is configured to fail.
    #[error(
        "clock moved backwards by {difference_millis} ms: last={last_timestamp}, current={current_timestamp}"
    )]
    ClockMovedBackwards {
        /// The last timestamp already handed out by the generator.
        last_timestamp: u64,
        /// The current wall-clock timestamp observed during generation.
        current_timestamp: u64,
        /// The observed rollback distance.
        difference_millis: u64,
    },
    /// The random string generator requires a non-empty alphabet.
    #[error("alphabet must not be empty")]
    EmptyAlphabet,
    /// The random string generator requires a non-zero output length.
    #[error("length must be greater than zero")]
    InvalidLength,
}
