//! The abstraction a back-off policy pauses through.

use std::sync::Arc;

use next_web_core::async_trait;
use tokio::time::{Duration, sleep};

use crate::error::retry_error::RetryError;

/// Pauses execution between two attempts.
///
/// The default implementation waits on the Tokio timer, which yields the worker instead of
/// blocking it: a retry that waits must never stall the runtime that is running it.
#[async_trait]
pub trait Sleeper
where
    Self: Send + Sync,
{
    /// Waits for `back_off_period` milliseconds.
    ///
    /// Returns [`RetryError::BackOffInterruptedError`] when the wait cannot complete, for
    /// example because a custom sleeper observed a shutdown request.
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError>;
}

/// Waits on the Tokio timer.
///
/// It replaces the blocking thread wait of the original implementation: blocking a worker
/// thread for a back off would starve every other task the runtime is driving.
#[derive(Clone, Copy, Debug, Default)]
pub struct ThreadWaitSleeper {
    _private: (),
}

impl ThreadWaitSleeper {
    /// Creates the sleeper.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Sleeper for ThreadWaitSleeper {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        sleep(Duration::from_millis(back_off_period)).await;
        Ok(())
    }
}

/// Waits on the Tokio timer.
///
/// This is the deprecated blocking sleeper of the original API. It is kept as a distinct
/// name so existing configuration keeps compiling, and it behaves exactly like
/// [`ThreadWaitSleeper`].
#[derive(Clone, Copy, Debug, Default)]
#[deprecated(note = "use ThreadWaitSleeper, which is the same implementation")]
pub struct ObjectWaitSleeper {
    _private: (),
}

#[allow(deprecated)]
impl ObjectWaitSleeper {
    /// Creates the sleeper.
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(deprecated)]
#[async_trait]
impl Sleeper for ObjectWaitSleeper {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        ThreadWaitSleeper::new().sleep(back_off_period).await
    }
}

/// A sleeper that does not wait at all.
///
/// Use it in tests and for simulations that must stay deterministic and fast.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImmediateSleeper {
    _private: (),
}

impl ImmediateSleeper {
    /// Creates the sleeper.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Sleeper for ImmediateSleeper {
    async fn sleep(&self, _back_off_period: u64) -> Result<(), RetryError> {
        Ok(())
    }
}

/// Records every requested wait and returns immediately.
///
/// Useful to assert what a policy asked for without making the test slow.
#[derive(Clone, Default)]
pub struct RecordingSleeper {
    periods: Arc<std::sync::Mutex<Vec<u64>>>,
}

impl RecordingSleeper {
    /// Creates an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the requested waits, in milliseconds.
    pub fn periods(&self) -> Vec<u64> {
        self.periods
            .lock()
            .map(|periods| periods.clone())
            .unwrap_or_default()
    }
}

#[async_trait]
impl Sleeper for RecordingSleeper {
    async fn sleep(&self, back_off_period: u64) -> Result<(), RetryError> {
        if let Ok(mut periods) = self.periods.lock() {
            periods.push(back_off_period);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{ImmediateSleeper, RecordingSleeper, Sleeper, ThreadWaitSleeper};

    #[tokio::test]
    async fn the_immediate_sleeper_does_not_wait() {
        let start = Instant::now();

        ImmediateSleeper::new()
            .sleep(10_000)
            .await
            .expect("the immediate sleeper cannot fail");

        assert!(start.elapsed().as_millis() < 100);
    }

    #[tokio::test]
    async fn the_recording_sleeper_collects_the_requested_periods() {
        let sleeper = RecordingSleeper::new();

        sleeper.sleep(10).await.expect("recording cannot fail");
        sleeper.sleep(20).await.expect("recording cannot fail");

        assert_eq!(sleeper.periods(), vec![10, 20]);
    }

    #[tokio::test]
    async fn the_thread_wait_sleeper_waits_on_the_timer() {
        let start = Instant::now();

        ThreadWaitSleeper::new()
            .sleep(20)
            .await
            .expect("the timer sleep cannot fail");

        assert!(start.elapsed().as_millis() >= 15);
    }
}
