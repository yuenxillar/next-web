use std::str::FromStr;

/// Represents a unit of time for expressing durations and delays.
/// providing standard time units from nanoseconds to days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    /// Time unit representing one billionth of a second (10⁻⁹ seconds).
    Nanoseconds,

    /// Time unit representing one millionth of a second (10⁻⁶ seconds).
    Microseconds,

    /// Time unit representing one thousandth of a second (10⁻³ seconds).
    Milliseconds,

    /// Time unit representing one second.
    Seconds,

    /// Time unit representing sixty seconds.
    Minutes,

    /// Time unit representing sixty minutes (3,600 seconds).
    Hours,

    /// Time unit representing twenty-four hours (86,400 seconds).
    Days,
}

impl FromStr for TimeUnit {
    
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "ns" | "nanoseconds" => Ok(TimeUnit::Nanoseconds),
            "us" | "microseconds" => Ok(TimeUnit::Microseconds),
            "ms" | "milliseconds" => Ok(TimeUnit::Milliseconds),
            "s"  | "seconds" => Ok(TimeUnit::Seconds),
            "m"  | "minutes" => Ok(TimeUnit::Minutes),
            "h"  | "hours" => Ok(TimeUnit::Hours),
            "d"  | "days" => Ok(TimeUnit::Days),
            _ => Err(format!("Invalid time unit string: '{}'", s)),
        }
    }
}


impl TimeUnit {
    pub fn to_duration(self, value: u64) -> std::time::Duration {
         match self {
            TimeUnit::Nanoseconds => std::time::Duration::from_nanos(value),
            TimeUnit::Microseconds => std::time::Duration::from_micros(value),
            TimeUnit::Milliseconds => std::time::Duration::from_millis(value),
            TimeUnit::Seconds => std::time::Duration::from_secs(value),
            TimeUnit::Minutes => std::time::Duration::from_secs(value * 60),
            TimeUnit::Hours => std::time::Duration::from_secs(value * 3600),
            TimeUnit::Days => std::time::Duration::from_secs(value * 86400),
        }
    }
}