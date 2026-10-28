use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};

/// Local date and time utility.
///
/// Provides a set of static methods for retrieving and formatting the current local date and time.
pub struct LocalDateTime;

impl LocalDateTime {
    /// Get the current `DateTime<Local>` instance.
    ///
    /// # Returns
    ///
    /// The current local date and time.
    fn _now() -> DateTime<Local> {
        Local::now()
    }

    /// Get the current date and time as a string (format: YYYY-MM-DD HH:MM:SS.fff).
    ///
    /// # Returns
    ///
    /// Formatted time string with millisecond precision.
    pub fn now() -> String {
        Self::_now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }

    /// Get the current Unix timestamp in seconds.
    ///
    /// # Returns
    ///
    /// Number of seconds since the Unix epoch (1970-01-01 00:00:00 UTC).
    pub fn timestamp() -> i64 {
        Self::_now().timestamp()
    }

    /// Get the current date as a string (format: YYYY-MM-DD).
    ///
    /// # Returns
    ///
    /// Formatted date string.
    pub fn date() -> String {
        Self::_now().format("%Y-%m-%d").to_string()
    }

    /// Get the current date formatted as a save path (format: YYYY/MM/DD).
    ///
    /// Commonly used for organizing logs or backups by date.
    ///
    /// # Returns
    ///
    /// Hierarchical date path string.
    pub fn save_path() -> String {
        Self::_now().format("%Y/%m/%d").to_string()
    }

    /// Get the current time as a string using a custom format.
    ///
    /// # Parameters
    ///
    /// * `format` - A format string compatible with the `chrono` crate.
    ///
    /// # Returns
    ///
    /// Formatted date and time string according to the provided format.
    pub fn with_format(format: &str) -> String {
        Self::_now().format(format).to_string()
    }

    /// Get the current millisecond timestamp.
    ///
    /// # Returns
    ///
    /// Number of milliseconds since the Unix epoch.
    pub fn timestamp_millis() -> i64 {
        Self::_now().timestamp_millis()
    }

    /// Get the current microsecond timestamp.
    ///
    /// # Returns
    ///
    /// Number of microseconds since the Unix epoch.
    pub fn timestamp_micros() -> i64 {
        Self::_now().timestamp_micros()
    }

    /// Get the current day of week in English (abbreviated).
    ///
    /// # Returns
    ///
    /// Abbreviated English day name: "Mon", "Tue", etc.
    pub fn day_of_week_en() -> String {
        Self::_now().format("%a").to_string()
    }

    /// Get the current year.
    ///
    /// # Returns
    ///
    /// The current year (e.g., 2025).
    pub fn year() -> i32 {
        Self::_now().year()
    }

    /// Get the current month.
    ///
    /// # Returns
    ///
    /// Month number between 1 and 12.
    pub fn month() -> u32 {
        Self::_now().month()
    }

    /// Get the current day of month.
    ///
    /// # Returns
    ///
    /// Day of the month (1-31).
    pub fn day() -> u32 {
        Self::_now().day()
    }

    /// Get the current hour (24-hour clock).
    ///
    /// # Returns
    ///
    /// Hour of the day (0-23).
    pub fn hour() -> u32 {
        Self::_now().hour()
    }

    /// Get the current minute.
    ///
    /// # Returns
    ///
    /// Minute of the hour (0-59).
    pub fn minute() -> u32 {
        Self::_now().minute()
    }

    /// Get the current second.
    ///
    /// # Returns
    ///
    /// Second of the minute (0-59, excluding leap seconds).
    pub fn second() -> u32 {
        Self::_now().second()
    }

    /// Parse a date string with a given format into `NaiveDate`.
    ///
    /// # Parameters
    ///
    /// * `date_str` - The date string to parse.
    /// * `format` - The format string used for parsing.
    ///
    /// # Returns
    ///
    /// `Some(NaiveDate)` on success, `None` on failure.
    pub fn parse_date(date_str: &str, format: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(date_str, format).ok()
    }

    /// Get the date N days before today.
    ///
    /// # Parameters
    ///
    /// * `days` - Number of days to subtract.
    ///
    /// # Returns
    ///
    /// Date string (format: YYYY-MM-DD) from N days ago.
    pub fn date_before(days: u64) -> String {
        let date = Self::_now().date_naive() - chrono::Duration::days(days as i64);
        date.format("%Y-%m-%d").to_string()
    }

    /// Get the date N days after today.
    ///
    /// # Parameters
    ///
    /// * `days` - Number of days to add.
    ///
    /// # Returns
    ///
    /// Date string (format: YYYY-MM-DD) from N days in the future.
    pub fn date_after(days: u64) -> String {
        let date = Self::_now().date_naive() + chrono::Duration::days(days as i64);
        date.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Local, NaiveDate, Timelike};

    // ---------- helpers ----------

    /// Validate the `YYYY-MM-DD HH:MM:SS.fff` format by inspecting bytes.
    fn is_now_format(s: &str) -> bool {
        // Length: 4+1+2+1+2+1+2+1+2+1+2+1+3 = 23
        if s.len() != 23 {
            return false;
        }
        let b = s.as_bytes();
        let digit = |i: usize| b[i].is_ascii_digit();
        (0..4).all(digit)
            && b[4] == b'-'
            && (5..7).all(digit)
            && b[7] == b'-'
            && (8..10).all(digit)
            && b[10] == b' '
            && (11..13).all(digit)
            && b[13] == b':'
            && (14..16).all(digit)
            && b[16] == b':'
            && (17..19).all(digit)
            && b[19] == b'.'
            && (20..23).all(digit)
    }

    /// Validate the `YYYY-MM-DD` format.
    fn is_date_format(s: &str) -> bool {
        s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
    }

    // ---------- now / format ----------

    #[test]
    fn now_has_expected_format() {
        let s = LocalDateTime::now();
        assert!(is_now_format(&s), "unexpected format: {s}");
    }

    #[test]
    fn now_is_close_to_system_time() {
        // Sample the system clock before and after the call, then assert
        // that now() falls inside the interval (5 ms tolerance).
        let before = Local::now().timestamp_millis();
        let s = LocalDateTime::now();
        let after = Local::now().timestamp_millis();

        let dt = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.3f")
            .expect("parse now()");
        let ts = dt.and_local_timezone(Local).unwrap().timestamp_millis();

        assert!(
            ts >= before - 5 && ts <= after + 5,
            "now() ts={ts} not in [{before}, {after}]"
        );
    }

    #[test]
    fn date_matches_now_prefix() {
        let now = LocalDateTime::now();
        let date = LocalDateTime::date();
        assert!(
            now.starts_with(&date),
            "date={date} not a prefix of now={now}"
        );
        assert!(is_date_format(&date));
    }

    #[test]
    fn save_path_format() {
        let p = LocalDateTime::save_path();
        // Expected shape: YYYY/MM/DD, length 10
        assert_eq!(p.len(), 10, "unexpected save_path: {p}");
        let parts: Vec<&str> = p.split('/').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 4);
        assert_eq!(parts[1].len(), 2);
        assert_eq!(parts[2].len(), 2);
        assert!(parts.iter().all(|s| s.parse::<u32>().is_ok()));
    }

    #[test]
    fn with_format_custom() {
        let s = LocalDateTime::with_format("%Y");
        let year = LocalDateTime::year();
        assert_eq!(s, year.to_string());
    }

    // ---------- timestamps ----------

    #[test]
    fn timestamp_and_millis_consistency() {
        // Within the same second: millis/1000 should equal seconds,
        // allowing a boundary difference of 1.
        let secs = LocalDateTime::timestamp();
        let millis = LocalDateTime::timestamp_millis();
        let diff = (millis / 1000 - secs).abs();
        assert!(diff <= 1, "secs={secs}, millis={millis}");
    }

    #[test]
    fn timestamp_is_reasonable() {
        let ts = LocalDateTime::timestamp();
        // Sane bounds: 2020-01-01 .. 2100-01-01
        assert!(ts > 1_577_836_800 && ts < 4_102_444_800, "ts={ts}");
    }

    // ---------- weekday / components ----------

    #[test]
    fn components_match_now() {
        let now = Local::now();
        assert_eq!(LocalDateTime::year(), now.year());
        assert_eq!(LocalDateTime::month(), now.month());
        assert_eq!(LocalDateTime::day(), now.day());
        assert_eq!(LocalDateTime::hour(), now.hour());
        assert_eq!(LocalDateTime::minute(), now.minute());
        assert_eq!(LocalDateTime::second(), now.second());
    }

    #[test]
    fn components_in_valid_ranges() {
        assert!((1..=12).contains(&LocalDateTime::month()));
        assert!((1..=31).contains(&LocalDateTime::day()));
        assert!((0..=23).contains(&LocalDateTime::hour()));
        assert!((0..=59).contains(&LocalDateTime::minute()));
        assert!((0..=59).contains(&LocalDateTime::second()));
    }

    // ---------- parse_date ----------

    #[test]
    fn parse_date_valid() {
        let d = LocalDateTime::parse_date("2025-03-14", "%Y-%m-%d").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2025, 3, 14).unwrap());
    }

    #[test]
    fn parse_date_invalid_returns_none() {
        assert!(LocalDateTime::parse_date("not-a-date", "%Y-%m-%d").is_none());
        assert!(LocalDateTime::parse_date("2025-13-01", "%Y-%m-%d").is_none());
        assert!(LocalDateTime::parse_date("2025-02-30", "%Y-%m-%d").is_none());
    }

    #[test]
    fn parse_date_with_custom_format() {
        let d = LocalDateTime::parse_date("14/03/2025", "%d/%m/%Y").unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2025, 3, 14).unwrap());
    }

    // ---------- date_before / date_after ----------

    #[test]
    fn date_before_and_after_zero_are_today() {
        let today = LocalDateTime::date();
        assert_eq!(LocalDateTime::date_before(0), today);
        assert_eq!(LocalDateTime::date_after(0), today);
    }

    #[test]
    fn date_before_n_days() {
        let expected = (Local::now().date_naive() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(LocalDateTime::date_before(7), expected);
    }

    #[test]
    fn date_after_n_days() {
        let expected = (Local::now().date_naive() + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(LocalDateTime::date_after(30), expected);
    }

    #[test]
    fn date_before_after_are_inverse() {
        // before(5) + 5 days == today == after(5) - 5 days
        let b = NaiveDate::parse_from_str(&LocalDateTime::date_before(5), "%Y-%m-%d").unwrap();
        let a = NaiveDate::parse_from_str(&LocalDateTime::date_after(5), "%Y-%m-%d").unwrap();
        assert_eq!((a - b).num_days(), 10);
    }
}
