//! `After` / `Before` / `Between` predicates over zoned timestamps.
//!
//! Values are trimmed, accept `start,end` and `start/end`, and understand the
//! Spring form `2017-01-20T17:42:47-07:00[America/Denver]`.

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// The time window predicate.
#[derive(Debug, Clone)]
pub enum ZonedDateTimeRoutePredicateFactory {
    /// Matches when now is at or after the timestamp.
    After(Vec<DateTime<FixedOffset>>),
    /// Matches when now is strictly before the timestamp.
    Before(Vec<DateTime<FixedOffset>>),
    /// Matches when now falls in `[start, end)`.
    Between(Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)>),
}

impl ZonedDateTimeRoutePredicateFactory {
    /// Whether the current time satisfies the window.
    pub fn matches(&self, _session: &mut pingora::protocols::http::ServerSession) -> bool {
        let now = Utc::now();

        match self {
            Self::After(targets) => targets
                .iter()
                .any(|target| now >= target.with_timezone(&Utc)),
            Self::Before(targets) => targets
                .iter()
                .any(|target| now < target.with_timezone(&Utc)),
            Self::Between(ranges) => ranges.iter().any(|(start, end)| {
                let start = start.with_timezone(&Utc);
                let end = end.with_timezone(&Utc);

                start < end && now >= start && now < end
            }),
        }
    }
}

/// Parse a timestamp, applying `[Region/Zone]` when no offset is present.
pub(crate) fn parse_zoned_datetime(raw: &str) -> Result<DateTime<FixedOffset>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("timestamp is empty".to_string());
    }

    let (stamp, zone) = match raw.split_once('[') {
        Some((stamp, rest)) => (stamp.trim(), rest.strip_suffix(']').map(str::trim)),
        None => (raw, None),
    };

    if let Ok(parsed) = DateTime::parse_from_rfc3339(stamp) {
        return Ok(parsed);
    }

    let zone = zone.ok_or_else(|| {
        format!("`{raw}` is not an RFC 3339 timestamp and carries no [Region/Zone] suffix")
    })?;
    let zone: Tz = zone
        .parse()
        .map_err(|_| format!("`{zone}` is not a known time zone"))?;

    let naive: NaiveDateTime = stamp
        .parse()
        .map_err(|error| format!("`{stamp}` is not a valid timestamp: {error}"))?;

    zone.from_local_datetime(&naive)
        .single()
        .map(|value| value.fixed_offset())
        .ok_or_else(|| format!("`{stamp}` is ambiguous or invalid in `{zone}`"))
}

pub(super) fn after(raw: &str) -> Result<ZonedDateTimeRoutePredicateFactory, String> {
    Ok(ZonedDateTimeRoutePredicateFactory::After(vec![
        parse_zoned_datetime(raw)?,
    ]))
}

pub(super) fn before(raw: &str) -> Result<ZonedDateTimeRoutePredicateFactory, String> {
    Ok(ZonedDateTimeRoutePredicateFactory::Before(vec![
        parse_zoned_datetime(raw)?,
    ]))
}

pub(super) fn between(
    start_raw: &str,
    end_raw: &str,
) -> Result<ZonedDateTimeRoutePredicateFactory, String> {
    let start = parse_zoned_datetime(start_raw)?;
    let end = parse_zoned_datetime(end_raw)?;

    if start >= end {
        return Err(format!("`{start_raw}` must be before `{end_raw}`"));
    }

    Ok(ZonedDateTimeRoutePredicateFactory::Between(vec![(
        start, end,
    )]))
}

#[cfg(test)]
mod tests {
    use super::{between, parse_zoned_datetime};

    #[test]
    fn value_is_trimmed() {
        let parsed = parse_zoned_datetime("  2024-01-01T00:00:00+00:00  ").unwrap();
        assert_eq!(parsed.to_rfc3339(), "2024-01-01T00:00:00+00:00");
    }

    #[test]
    fn region_suffix_is_accepted() {
        let parsed = parse_zoned_datetime("2017-01-20T17:42:47-07:00[America/Denver]").unwrap();
        assert_eq!(parsed.offset().local_minus_utc(), -7 * 3600);

        let parsed = parse_zoned_datetime("2017-01-20T17:42:47[America/Denver]").unwrap();
        assert_eq!(parsed.offset().local_minus_utc(), -7 * 3600);
    }

    #[test]
    fn unknown_region_is_an_error() {
        assert!(parse_zoned_datetime("2017-01-20T17:42:47[Mars/Olympus]").is_err());
    }

    #[test]
    fn between_rejects_an_inverted_window() {
        assert!(between("2024-02-01T00:00:00+00:00", "2024-01-01T00:00:00+00:00").is_err());
    }

    #[test]
    fn between_accepts_a_valid_window() {
        let predicate = between("2024-01-01T00:00:00+00:00", "2024-02-01T00:00:00+00:00").unwrap();
        assert!(matches!(
            predicate,
            super::ZonedDateTimeRoutePredicateFactory::Between(_)
        ));
    }
}
