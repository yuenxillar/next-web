use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::IdGeneratorError;
use crate::generator::IdGenerator;

const DEFAULT_EPOCH_MILLIS: u64 = 1_577_836_800_000;
const NODE_ID_BITS: u8 = 10;
const SEQUENCE_BITS: u8 = 12;
const MAX_NODE_ID: u16 = (1u16 << NODE_ID_BITS) - 1;
const MAX_SEQUENCE: u16 = (1u16 << SEQUENCE_BITS) - 1;
const NODE_ID_SHIFT: u8 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: u8 = NODE_ID_BITS + SEQUENCE_BITS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SnowflakeState {
    last_timestamp: u64,
    sequence: u16,
}

/// Strategy used when the system clock goes backwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClockRollbackStrategy {
    /// Wait until the last observed millisecond catches up.
    #[default]
    Wait,
    /// Return an error immediately.
    Error,
}

/// Configures the snowflake generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnowflakeConfig {
    node_id: u16,
    epoch_millis: u64,
    rollback_strategy: ClockRollbackStrategy,
}

impl SnowflakeConfig {
    /// Creates a config using the default epoch `2020-01-01T00:00:00Z`.
    pub const fn new(node_id: u16) -> Self {
        Self {
            node_id,
            epoch_millis: DEFAULT_EPOCH_MILLIS,
            rollback_strategy: ClockRollbackStrategy::Wait,
        }
    }

    /// Sets the custom epoch in milliseconds.
    pub const fn with_epoch_millis(mut self, epoch_millis: u64) -> Self {
        self.epoch_millis = epoch_millis;
        self
    }

    /// Sets the clock rollback strategy.
    pub const fn with_rollback_strategy(
        mut self,
        rollback_strategy: ClockRollbackStrategy,
    ) -> Self {
        self.rollback_strategy = rollback_strategy;
        self
    }

    /// Returns the configured node id.
    pub const fn node_id(self) -> u16 {
        self.node_id
    }

    /// Returns the configured epoch.
    pub const fn epoch_millis(self) -> u64 {
        self.epoch_millis
    }

    /// Returns the rollback strategy.
    pub const fn rollback_strategy(self) -> ClockRollbackStrategy {
        self.rollback_strategy
    }

    fn validate(self) -> Result<Self, IdGeneratorError> {
        if self.node_id > MAX_NODE_ID {
            return Err(IdGeneratorError::InvalidNodeId {
                node_id: self.node_id,
                max_node_id: MAX_NODE_ID,
            });
        }
        if self.epoch_millis > current_time_millis() {
            return Err(IdGeneratorError::EpochInFuture {
                epoch_millis: self.epoch_millis,
            });
        }
        Ok(self)
    }
}

impl Default for SnowflakeConfig {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A parsed snowflake ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnowflakeId {
    raw: u64,
}

impl SnowflakeId {
    /// Creates a parsed view from a raw snowflake ID.
    pub const fn from_raw(raw: u64) -> Self {
        Self { raw }
    }

    /// Returns the original integer representation.
    pub const fn raw(self) -> u64 {
        self.raw
    }

    /// Returns the absolute timestamp in milliseconds.
    pub const fn timestamp_millis(self, epoch_millis: u64) -> u64 {
        (self.raw >> TIMESTAMP_SHIFT) + epoch_millis
    }

    /// Returns the configured node id stored in the ID.
    pub const fn node_id(self) -> u16 {
        ((self.raw >> NODE_ID_SHIFT) & (MAX_NODE_ID as u64)) as u16
    }

    /// Returns the per-millisecond sequence number.
    pub const fn sequence(self) -> u16 {
        (self.raw & (MAX_SEQUENCE as u64)) as u16
    }
}

/// A 64-bit snowflake generator suitable for distributed deployments.
#[derive(Debug)]
pub struct SnowflakeGenerator {
    config: SnowflakeConfig,
    state: Mutex<SnowflakeState>,
}

impl SnowflakeGenerator {
    /// Creates a generator with the default epoch and wait-on-rollback behavior.
    pub fn new(node_id: u16) -> Result<Self, IdGeneratorError> {
        Self::with_config(SnowflakeConfig::new(node_id))
    }

    /// Creates a generator from the provided config.
    pub fn with_config(config: SnowflakeConfig) -> Result<Self, IdGeneratorError> {
        Ok(Self {
            config: config.validate()?,
            state: Mutex::new(SnowflakeState {
                last_timestamp: 0,
                sequence: 0,
            }),
        })
    }

    /// Returns the generator config.
    pub const fn config(&self) -> SnowflakeConfig {
        self.config
    }

    /// Parses a raw snowflake ID according to this generator's config.
    pub fn parse(&self, raw: u64) -> SnowflakeId {
        SnowflakeId::from_raw(raw)
    }

    fn next_timestamp_after(last_timestamp: u64) -> u64 {
        let mut current = current_time_millis();
        while current <= last_timestamp {
            std::hint::spin_loop();
            current = current_time_millis();
        }
        current
    }
}

impl IdGenerator<u64> for SnowflakeGenerator {
    fn next_id(&self) -> Result<u64, IdGeneratorError> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut current_timestamp = current_time_millis();

        if current_timestamp < state.last_timestamp {
            match self.config.rollback_strategy() {
                ClockRollbackStrategy::Wait => {
                    current_timestamp = Self::next_timestamp_after(state.last_timestamp);
                }
                ClockRollbackStrategy::Error => {
                    return Err(IdGeneratorError::ClockMovedBackwards {
                        last_timestamp: state.last_timestamp,
                        current_timestamp,
                        difference_millis: state.last_timestamp - current_timestamp,
                    });
                }
            }
        }

        if current_timestamp == state.last_timestamp {
            state.sequence = (state.sequence + 1) & MAX_SEQUENCE;
            if state.sequence == 0 {
                current_timestamp = Self::next_timestamp_after(state.last_timestamp);
            }
        } else {
            state.sequence = 0;
        }

        state.last_timestamp = current_timestamp;

        let relative_timestamp = current_timestamp.saturating_sub(self.config.epoch_millis());
        let raw = (relative_timestamp << TIMESTAMP_SHIFT)
            | (u64::from(self.config.node_id()) << NODE_ID_SHIFT)
            | u64::from(state.sequence);

        Ok(raw)
    }
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
