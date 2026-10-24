use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc,
};

pub struct DateTimeUtil;

impl DateTimeUtil {
    #[must_use]
    pub fn now() -> String {
        Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }

    #[must_use]
    pub fn timestamp() -> i64 {
        Local::now().timestamp_millis()
    }

    #[must_use]
    pub fn date() -> String {
        Local::now().format("%Y-%m-%d").to_string()
    }

    #[must_use]
    pub fn time() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    #[must_use]
    pub fn format_now(fmt: &str) -> String {
        Local::now().format(fmt).to_string()
    }

    #[must_use]
    pub fn format(dt: &NaiveDateTime, fmt: &str) -> String {
        dt.format(fmt).to_string()
    }

    #[must_use]
    pub fn from_timestamp(timestamp: i64, fmt: &str) -> String {
        let dt = Local
            .timestamp_opt(timestamp, 0)
            .single()
            .unwrap_or_else(Local::now);
        dt.format(fmt).to_string()
    }

    #[must_use]
    pub fn from_timestamp_millis(timestamp_ms: i64, fmt: &str) -> String {
        let seconds = timestamp_ms.div_euclid(1000);
        let nanos = timestamp_ms.rem_euclid(1000) as u32 * 1_000_000;
        let dt = Local
            .timestamp_opt(seconds, nanos)
            .single()
            .unwrap_or_else(Local::now);
        dt.format(fmt).to_string()
    }

    #[must_use]
    pub fn parse(datetime_str: &str, fmt: &str) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(datetime_str.trim(), fmt).ok()
    }

    #[must_use]
    pub fn parse_date(date_str: &str, fmt: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(date_str.trim(), fmt).ok()
    }

    #[must_use]
    pub fn is_same_day(dt1: &NaiveDateTime, dt2: &NaiveDateTime) -> bool {
        dt1.date() == dt2.date()
    }

    #[must_use]
    pub fn add_days(dt: &NaiveDateTime, days: i64) -> NaiveDateTime {
        *dt + Duration::days(days)
    }

    #[must_use]
    pub fn add_hours(dt: &NaiveDateTime, hours: i64) -> NaiveDateTime {
        *dt + Duration::hours(hours)
    }

    #[must_use]
    pub fn add_minutes(dt: &NaiveDateTime, minutes: i64) -> NaiveDateTime {
        *dt + Duration::minutes(minutes)
    }

    #[must_use]
    pub fn add_seconds(dt: &NaiveDateTime, seconds: i64) -> NaiveDateTime {
        *dt + Duration::seconds(seconds)
    }

    #[must_use]
    pub fn days_between(dt1: &NaiveDateTime, dt2: &NaiveDateTime) -> i64 {
        (*dt2 - *dt1).num_days()
    }

    #[must_use]
    pub fn seconds_between(dt1: &NaiveDateTime, dt2: &NaiveDateTime) -> i64 {
        (*dt2 - *dt1).num_seconds()
    }

    #[must_use]
    pub fn first_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, month, 1)
    }

    #[must_use]
    pub fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
        let first_of_next_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        };

        first_of_next_month.and_then(|date| date.pred_opt())
    }

    #[must_use]
    pub fn is_weekday(dt: &NaiveDateTime) -> bool {
        dt.weekday().number_from_monday() <= 5
    }

    #[must_use]
    pub fn is_weekend(dt: &NaiveDateTime) -> bool {
        !Self::is_weekday(dt)
    }

    #[must_use]
    pub fn to_utc(dt: DateTime<Local>) -> DateTime<Utc> {
        dt.with_timezone(&Utc)
    }

    #[must_use]
    pub fn change_timezone(dt: &DateTime<Utc>, hours: i32) -> DateTime<FixedOffset> {
        let seconds = hours.clamp(-23, 23) * 3600;
        let offset = FixedOffset::east_opt(seconds).expect("clamped timezone offset is valid");
        dt.with_timezone(&offset)
    }

    #[must_use]
    pub fn get_quarter(dt: &NaiveDateTime) -> u32 {
        (dt.month() - 1) / 3 + 1
    }

    #[must_use]
    pub fn calculate_age(birth_date: &NaiveDate) -> u32 {
        let today = Local::now().date_naive();
        let mut age = today.year() - birth_date.year();

        if (today.month(), today.day()) < (birth_date.month(), birth_date.day()) {
            age -= 1;
        }

        age.max(0) as u32
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate};

    use super::DateTimeUtil;

    #[test]
    fn parse_and_add_work() {
        let dt = DateTimeUtil::parse("2024-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let shifted = DateTimeUtil::add_hours(&dt, 2);
        assert_eq!(DateTimeUtil::seconds_between(&dt, &shifted), 2 * 60 * 60);
    }

    #[test]
    fn month_boundaries_work() {
        assert_eq!(DateTimeUtil::last_day_of_month(2024, 2).unwrap().day(), 29);
        assert_eq!(
            DateTimeUtil::first_day_of_month(2024, 2).unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()
        );
    }
}
