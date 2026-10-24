use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::IdGeneratorError;
use crate::generator::IdGenerator;

/// A lock-free generator that returns strictly increasing `u64` IDs per process.
///
/// The generated values are time-biased: the current millisecond timestamp is used as
/// the preferred base, and an atomic compare-exchange guarantees monotonicity even
/// when many threads request IDs within the same millisecond.
#[derive(Debug, Default)]
pub struct MonotonicIdGenerator {
    last_issued: AtomicU64,
}

impl MonotonicIdGenerator {
    /// Creates a new generator starting from the current wall-clock time.
    pub fn new() -> Self {
        Self::with_seed(Self::time_seed())
    }

    /// Creates a generator with an explicit starting seed.
    pub const fn with_seed(seed: u64) -> Self {
        Self {
            last_issued: AtomicU64::new(seed),
        }
    }

    /// Returns the current generator cursor.
    pub fn current(&self) -> u64 {
        self.last_issued.load(Ordering::Relaxed)
    }

    fn time_seed() -> u64 {
        current_time_millis().saturating_mul(1_000)
    }
}

impl IdGenerator<u64> for MonotonicIdGenerator {
    fn next_id(&self) -> Result<u64, IdGeneratorError> {
        let candidate = Self::time_seed();

        loop {
            let current = self.last_issued.load(Ordering::Relaxed);
            let next = candidate.max(current.saturating_add(1));

            match self.last_issued.compare_exchange(
                current,
                next,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Ok(next),
                Err(_) => continue,
            }
        }
    }
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
