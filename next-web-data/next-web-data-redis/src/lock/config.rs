//! Runtime configuration for distributed locks.

use std::time::Duration;

use super::error::RedisLockError;

/// Timing knobs shared by every lock handle built from one service.
///
/// The defaults match Redisson's `lockWatchdogTimeout`: a 30-second watchdog
/// lease renewed every ten seconds, a 100-millisecond retry delay while a
/// contender waits, and a 3-second budget for one backend call.
#[derive(Debug, Clone)]
pub struct RedisLockConfig {
    /// The lease a watchdog lock is granted, and the unit it is renewed in.
    pub watchdog_timeout: Duration,
    /// The delay between two acquisition attempts while a caller waits.
    pub retry_interval: Duration,
    /// The budget for a single backend call, after which it is abandoned.
    pub command_timeout: Duration,
}

impl Default for RedisLockConfig {
    fn default() -> Self {
        Self {
            watchdog_timeout: Duration::from_secs(30),
            retry_interval: Duration::from_millis(100),
            command_timeout: Duration::from_secs(3),
        }
    }
}

impl RedisLockConfig {
    /// Validate every duration.
    ///
    /// # Errors
    ///
    /// Returns [`RedisLockError::InvalidDuration`] naming the offending field
    /// when a duration is zero or cannot be expressed in milliseconds, and
    /// [`RedisLockError::InvalidConfiguration`] when `command_timeout` does not
    /// fit inside one watchdog renewal interval.
    pub fn validate(&self) -> Result<(), RedisLockError> {
        for (field, value) in [
            ("watchdog_timeout", self.watchdog_timeout),
            ("retry_interval", self.retry_interval),
            ("command_timeout", self.command_timeout),
        ] {
            if value.is_zero() || value.as_millis() > u64::MAX as u128 {
                return Err(RedisLockError::InvalidDuration { field });
            }
        }

        // The watchdog renews every `watchdog_timeout / 3` and may spend two
        // command timeouts on a single renewal. Anything slower than one
        // interval can let the lease expire while the guard still believes it
        // holds the lock, which is exactly the mutual exclusion the lock exists
        // to provide.
        if self.command_timeout > self.watchdog_timeout / 3 {
            return Err(RedisLockError::InvalidConfiguration(format!(
                "command_timeout ({:?}) must not exceed a third of watchdog_timeout ({:?}): \
                 a renewal that cannot finish inside one renewal interval may let the lease \
                 expire while the guard still holds it",
                self.command_timeout, self.watchdog_timeout
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_redisson_watchdog() {
        let config = RedisLockConfig::default();
        assert_eq!(config.watchdog_timeout, Duration::from_secs(30));
        assert_eq!(config.retry_interval, Duration::from_millis(100));
        assert_eq!(config.command_timeout, Duration::from_secs(3));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validation_names_the_offending_field() {
        let zero_retry = RedisLockConfig {
            retry_interval: Duration::ZERO,
            ..Default::default()
        };
        assert!(matches!(
            zero_retry.validate(),
            Err(RedisLockError::InvalidDuration {
                field: "retry_interval"
            })
        ));

        let too_long = RedisLockConfig {
            command_timeout: Duration::from_millis(u64::MAX).saturating_add(Duration::from_secs(1)),
            ..Default::default()
        };
        assert!(matches!(
            too_long.validate(),
            Err(RedisLockError::InvalidDuration {
                field: "command_timeout"
            })
        ));
    }

    #[test]
    fn a_command_timeout_that_outlives_a_renewal_interval_is_rejected() {
        // The watchdog renews every watchdog_timeout/3 and may spend two
        // command timeouts on one renewal. If that budget exceeds the interval,
        // the lease can expire while the guard still reports that it holds it.
        let config = RedisLockConfig {
            watchdog_timeout: Duration::from_secs(1),
            command_timeout: Duration::from_secs(3),
            ..Default::default()
        };

        assert!(matches!(
            config.validate(),
            Err(RedisLockError::InvalidConfiguration(message))
                if message.contains("command_timeout") && message.contains("watchdog_timeout")
        ));

        // The defaults keep a third of the watchdog timeout as headroom.
        let safe = RedisLockConfig {
            watchdog_timeout: Duration::from_secs(30),
            command_timeout: Duration::from_secs(3),
            ..Default::default()
        };
        assert!(safe.validate().is_ok());
    }
}
