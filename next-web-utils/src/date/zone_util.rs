use chrono::{DateTime, FixedOffset, Local, LocalResult, NaiveDateTime, TimeZone, Utc};

pub struct ZoneUtil;

impl ZoneUtil {
    #[must_use]
    pub fn utc_now() -> DateTime<Utc> {
        Utc::now()
    }

    #[must_use]
    pub fn local_now() -> DateTime<Local> {
        Local::now()
    }

    #[must_use]
    pub fn fixed_offset(hours: i32) -> Option<FixedOffset> {
        FixedOffset::east_opt(hours.checked_mul(3600)?)
    }

    #[must_use]
    pub fn fixed_offset_seconds(seconds: i32) -> Option<FixedOffset> {
        FixedOffset::east_opt(seconds)
    }

    #[must_use]
    pub fn local_to_utc(datetime: DateTime<Local>) -> DateTime<Utc> {
        datetime.with_timezone(&Utc)
    }

    #[must_use]
    pub fn utc_to_local(datetime: DateTime<Utc>) -> DateTime<Local> {
        datetime.with_timezone(&Local)
    }

    #[must_use]
    pub fn to_fixed_offset(datetime: DateTime<Utc>, offset: FixedOffset) -> DateTime<FixedOffset> {
        datetime.with_timezone(&offset)
    }

    #[must_use]
    pub fn parse_rfc3339(value: &str) -> Option<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(value.trim()).ok()
    }

    #[must_use]
    pub fn parse_with_offset(
        value: &str,
        format: &str,
        offset: FixedOffset,
    ) -> Option<DateTime<FixedOffset>> {
        let naive = NaiveDateTime::parse_from_str(value.trim(), format).ok()?;
        match offset.from_local_datetime(&naive) {
            LocalResult::Single(value) => Some(value),
            LocalResult::Ambiguous(first, _) => Some(first),
            LocalResult::None => None,
        }
    }

    #[must_use]
    pub fn format_with_offset(
        datetime: DateTime<Utc>,
        offset: FixedOffset,
        format: &str,
    ) -> String {
        datetime.with_timezone(&offset).format(format).to_string()
    }

    #[must_use]
    pub fn offset_hours(datetime: DateTime<FixedOffset>) -> i32 {
        datetime.offset().local_minus_utc() / 3600
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Timelike, Utc};

    use super::ZoneUtil;

    #[test]
    fn fixed_offset_conversion_works() {
        let utc = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let offset = ZoneUtil::fixed_offset(8).unwrap();
        let shifted = ZoneUtil::to_fixed_offset(utc, offset);

        assert_eq!(shifted.hour(), 8);
        assert_eq!(ZoneUtil::offset_hours(shifted), 8);
    }

    #[test]
    fn rfc3339_parse_works() {
        let parsed = ZoneUtil::parse_rfc3339("2024-01-01T00:00:00+08:00").unwrap();
        assert_eq!(ZoneUtil::offset_hours(parsed), 8);
    }
}
