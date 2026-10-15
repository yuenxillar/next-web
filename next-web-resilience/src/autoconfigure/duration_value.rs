//! Durations as they are written in a configuration file.

use std::{fmt, time::Duration};

/// A duration written in a configuration file.
///
/// The property binder has no way to read a `std::time::Duration` from `1s` or
/// from a bare number — serde's own implementation expects a `{ secs, nanos }`
/// mapping, which no configuration file in this project writes — so this is the
/// type the settings use instead, parsed by [`DurationValue::to_duration`].
///
/// It follows the shape already used for byte sizes: an untagged enum, so a
/// scalar and a string both bind. Every scalar the binder reads arrives as a
/// string, so a bare number takes the `Text` arm and is parsed as milliseconds.
///
/// ```
/// use next_web_resilience::autoconfigure::DurationValue;
///
/// assert_eq!(DurationValue::Text("250ms".to_owned()).to_duration().unwrap().as_millis(), 250);
/// assert_eq!(DurationValue::Millis(1_500).to_duration().unwrap().as_millis(), 1_500);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(untagged)]
pub enum DurationValue {
    /// A whole number of milliseconds.
    ///
    /// Written bare — `timeoutDuration: 0` — this is what "wait no time at all"
    /// looks like, and the same reading as `0` in the text form.
    Millis(u64),
    /// A number with a unit: `ms`, `s`, `m` or `h`.
    Text(String),
}

impl DurationValue {
    /// Reads the value as a [`Duration`].
    ///
    /// # Errors
    ///
    /// Returns [`DurationParseError`] when the text is not a number, carries no
    /// unit this understands, or is too large to hold.
    pub fn to_duration(&self) -> Result<Duration, DurationParseError> {
        let millis = match self {
            Self::Millis(millis) => *millis,
            Self::Text(text) => parse_millis(text)?,
        };

        Ok(Duration::from_millis(millis))
    }

    /// Reads the value as a whole number of milliseconds.
    ///
    /// # Errors
    ///
    /// As [`DurationValue::to_duration`], plus when the value does not divide
    /// evenly into milliseconds.
    pub fn to_millis(&self) -> Result<u64, DurationParseError> {
        match self {
            Self::Millis(millis) => Ok(*millis),
            Self::Text(text) => parse_millis(text),
        }
    }
}

impl fmt::Display for DurationValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Millis(millis) => write!(f, "{millis}ms"),
            Self::Text(text) => f.write_str(text),
        }
    }
}

/// Reads `250ms`, `2s`, `5m`, `1h` or a bare number of milliseconds.
///
/// The units and the bare-number reading match the parsers already used
/// elsewhere in the framework, so a duration means the same thing wherever it is
/// written.
fn parse_millis(text: &str) -> Result<u64, DurationParseError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(DurationParseError::new(text, "it is empty"));
    }

    // `ms` before `s`, or `10ms` would be read as ten of something else.
    let (number, multiplier) = if let Some(number) = text.strip_suffix("ms") {
        (number, 1_u64)
    } else if let Some(number) = text.strip_suffix('s') {
        (number, 1_000)
    } else if let Some(number) = text.strip_suffix('m') {
        (number, 60_000)
    } else if let Some(number) = text.strip_suffix('h') {
        (number, 3_600_000)
    } else {
        (text, 1)
    };

    // Underscores are allowed between digits, as they are in written byte sizes.
    let number: String = number.trim().chars().filter(|c| *c != '_').collect();
    let value: u64 = number
        .parse()
        .map_err(|_| DurationParseError::new(text, "that is not a number followed by ms, s, m or h"))?;

    value
        .checked_mul(multiplier)
        .ok_or_else(|| DurationParseError::new(text, "it is too large"))
}

/// A duration in a configuration file that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurationParseError {
    /// The text that was written.
    value: String,
    /// Why it could not be read.
    reason: String,
}

impl DurationParseError {
    fn new(value: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Returns the text that could not be read.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns why it could not be read.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for DurationParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a duration: {}", self.value, self.reason)
    }
}

impl std::error::Error for DurationParseError {}

#[cfg(test)]
mod tests {
    use super::DurationValue;

    fn millis(text: &str) -> u64 {
        DurationValue::Text(text.to_owned())
            .to_duration()
            .unwrap_or_else(|error| panic!("{text} should parse: {error}"))
            .as_millis() as u64
    }

    #[test]
    fn every_unit_is_read_the_way_it_is_written() {
        assert_eq!(millis("0"), 0);
        assert_eq!(millis("250ms"), 250);
        assert_eq!(millis("1s"), 1_000);
        assert_eq!(millis("10ms"), 10);
        assert_eq!(millis("2s"), 2_000);
        assert_eq!(millis("5m"), 300_000);
        assert_eq!(millis("1h"), 3_600_000);
    }

    #[test]
    fn the_text_is_read_loosely_but_not_guessed_at() {
        assert_eq!(millis("  2s  "), 2_000);
        assert_eq!(millis("1_000ms"), 1_000);

        for bad in ["", "  ", "abc", "s", "2 seconds", "-1s", "1.5s"] {
            assert!(
                DurationValue::Text(bad.to_owned()).to_duration().is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    #[test]
    fn a_value_too_large_to_hold_is_refused_rather_than_wrapped() {
        assert!(DurationValue::Text(format!("{}h", u64::MAX)).to_duration().is_err());
    }

    #[test]
    fn a_bare_number_is_milliseconds() {
        assert_eq!(DurationValue::Millis(0).to_duration().unwrap().as_millis(), 0);
        assert_eq!(
            DurationValue::Millis(1_500).to_duration().unwrap().as_millis(),
            1_500
        );
    }
}
