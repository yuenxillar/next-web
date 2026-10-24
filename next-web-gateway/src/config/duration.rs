//! Duration parsing for configuration values.
//!
//! Configurations accept human readable spans (`500ms`, `30s`, `2m`, `1h`).
//! A bare integer is interpreted as milliseconds for backwards compatibility
//! with the pre-`config` schema, and logs a deprecation warning.

use std::time::Duration;

use serde::{Deserialize, Deserializer};

/// Parse a duration such as `500ms`, `30s`, `2m` or `1h`.
///
/// A bare integer is treated as milliseconds and logs a deprecation warning.
pub fn parse_duration(raw: &str) -> Result<Duration, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("duration is empty".to_owned());
    }

    let (number, unit) = split_number_and_unit(text)?;
    let value = number
        .parse::<u64>()
        .map_err(|error| format!("invalid duration number `{number}`: {error}"))?;

    match unit {
        "ms" => Ok(Duration::from_millis(value)),
        "s" => Ok(Duration::from_secs(value)),
        "m" => Ok(Duration::from_secs(value.saturating_mul(60))),
        "h" => Ok(Duration::from_secs(value.saturating_mul(3_600))),
        other => Err(format!(
            "unknown duration unit `{other}`, expected one of ms, s, m, h"
        )),
    }
}

/// Split `30s` into `("30", "s")`, and a bare `30` into `("30", "ms")`.
fn split_number_and_unit(text: &str) -> Result<(&str, &'static str), String> {
    let end = text
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(text.len());

    if end == 0 {
        return Err(format!("duration `{text}` does not start with a number"));
    }

    let (number, unit) = text.split_at(end);
    match unit {
        "" => {
            tracing::warn!("duration `{text}` has no unit, assuming milliseconds");
            Ok((number, "ms"))
        }
        "ms" => Ok((number, "ms")),
        "s" => Ok((number, "s")),
        "m" => Ok((number, "m")),
        "h" => Ok((number, "h")),
        other => Err(format!(
            "unknown duration unit `{other}`, expected one of ms, s, m, h"
        )),
    }
}

/// Deserialize a duration field using [`parse_duration`].
pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_duration(&raw).map_err(serde::de::Error::custom)
}

/// Deserialize an optional duration field using [`parse_duration`].
pub(crate) fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    match raw {
        Some(raw) => parse_duration(&raw)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_duration;

    #[test]
    fn bare_number_is_milliseconds() {
        assert_eq!(parse_duration("250").unwrap().as_millis(), 250);
    }

    #[test]
    fn rejects_empty_and_unitless_garbage() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("s").is_err());
        assert!(parse_duration("10x").is_err());
    }
}
