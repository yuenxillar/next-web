use chrono::{
    DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone, Timelike,
};
use once_cell::sync::Lazy;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateUnit {
    MS,
    SECOND,
    MINUTE,
    HOUR,
    DAY,
    WEEK,
    MONTH,
    YEAR,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    YEAR,
    MONTH,
    DAY,
    HOUR,
    MINUTE,
    SECOND,
    MILLISECOND,
}

pub const CHINESE_ZODIAC: [&str; 12] = [
    "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Goat", "Monkey", "Rooster", "Dog",
    "Pig",
];

pub const ZODIAC: [&str; 12] = [
    "Aquarius",
    "Pisces",
    "Aries",
    "Taurus",
    "Gemini",
    "Cancer",
    "Leo",
    "Virgo",
    "Libra",
    "Scorpio",
    "Sagittarius",
    "Capricorn",
];

const ZODIAC_DATE: [u32; 12] = [20, 19, 21, 20, 21, 22, 23, 23, 23, 24, 23, 22];

static DATE_TIME_FORMATS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y.%m.%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y%m%d%H%M%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
    ]
});

static DATE_FORMATS: Lazy<Vec<&'static str>> =
    Lazy::new(|| vec!["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d", "%Y%m%d"]);

static TIME_FORMATS: Lazy<Vec<&'static str>> =
    Lazy::new(|| vec!["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"]);

pub struct DateUtil;

impl DateUtil {
    #[must_use]
    pub fn date() -> DateTime<Local> {
        Local::now()
    }

    #[must_use]
    pub fn date_ms(timestamp_ms: i64) -> DateTime<Local> {
        let seconds = timestamp_ms.div_euclid(1000);
        let millis = timestamp_ms.rem_euclid(1000) as u32;
        local_timestamp(seconds, millis * 1_000_000)
    }

    #[must_use]
    pub fn date_seconds(timestamp_seconds: i64) -> DateTime<Local> {
        local_timestamp(timestamp_seconds, 0)
    }

    #[must_use]
    pub fn current_millis() -> i64 {
        Local::now().timestamp_millis()
    }

    #[must_use]
    pub fn now() -> String {
        Self::format(Self::date(), "%Y-%m-%d %H:%M:%S")
    }

    #[must_use]
    pub fn today() -> String {
        Self::format(Self::date(), "%Y-%m-%d")
    }

    #[must_use]
    pub fn parse(date_str: &str) -> Option<DateTime<Local>> {
        let value = date_str.trim();

        if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
            return Some(dt.with_timezone(&Local));
        }
        if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
            return Some(dt.with_timezone(&Local));
        }

        for format in DATE_TIME_FORMATS.iter() {
            if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
                return local_from_naive(dt);
            }
        }

        for format in DATE_FORMATS.iter() {
            if let Ok(date) = NaiveDate::parse_from_str(value, format) {
                return local_from_naive(date.and_hms_opt(0, 0, 0)?);
            }
        }

        for format in TIME_FORMATS.iter() {
            if let Ok(time) = NaiveTime::parse_from_str(value, format) {
                return local_from_naive(Local::now().date_naive().and_time(time));
            }
        }

        None
    }

    #[must_use]
    pub fn parse_with_format(date_str: &str, format: &str) -> Option<DateTime<Local>> {
        let value = date_str.trim();
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
            return local_from_naive(dt);
        }
        if let Ok(date) = NaiveDate::parse_from_str(value, format) {
            return local_from_naive(date.and_hms_opt(0, 0, 0)?);
        }
        if let Ok(time) = NaiveTime::parse_from_str(value, format) {
            return local_from_naive(Local::now().date_naive().and_time(time));
        }
        None
    }

    #[must_use]
    pub fn format(date: DateTime<Local>, fmt: &str) -> String {
        date.format(fmt).to_string()
    }

    #[must_use]
    pub fn format_date(date: DateTime<Local>) -> String {
        Self::format(date, "%Y-%m-%d")
    }

    #[must_use]
    pub fn format_datetime(date: DateTime<Local>) -> String {
        Self::format(date, "%Y-%m-%d %H:%M:%S")
    }

    #[must_use]
    pub fn format_time(date: DateTime<Local>) -> String {
        Self::format(date, "%H:%M:%S")
    }

    #[must_use]
    pub fn year(date: DateTime<Local>) -> i32 {
        date.year()
    }

    #[must_use]
    pub fn month(date: DateTime<Local>) -> u32 {
        date.month()
    }

    #[must_use]
    pub fn day(date: DateTime<Local>) -> u32 {
        date.day()
    }

    #[must_use]
    pub fn hour(date: DateTime<Local>) -> u32 {
        date.hour()
    }

    #[must_use]
    pub fn minute(date: DateTime<Local>) -> u32 {
        date.minute()
    }

    #[must_use]
    pub fn second(date: DateTime<Local>) -> u32 {
        date.second()
    }

    #[must_use]
    pub fn day_of_week(date: DateTime<Local>) -> u32 {
        date.weekday().number_from_monday()
    }

    #[must_use]
    pub fn begin_of_day(date: DateTime<Local>) -> DateTime<Local> {
        local_from_naive(
            date.date_naive()
                .and_hms_milli_opt(0, 0, 0, 0)
                .expect("valid start of day"),
        )
        .expect("local start of day should exist")
    }

    #[must_use]
    pub fn end_of_day(date: DateTime<Local>) -> DateTime<Local> {
        local_from_naive(
            date.date_naive()
                .and_hms_milli_opt(23, 59, 59, 999)
                .expect("valid end of day"),
        )
        .expect("local end of day should exist")
    }

    #[must_use]
    pub fn begin_of_month(date: DateTime<Local>) -> DateTime<Local> {
        let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
            .expect("existing date has valid month");
        local_from_naive(first.and_hms_opt(0, 0, 0).expect("valid time"))
            .expect("local month start should exist")
    }

    #[must_use]
    pub fn end_of_month(date: DateTime<Local>) -> DateTime<Local> {
        let last_day = Self::days_in_month(date.year(), date.month());
        let last = NaiveDate::from_ymd_opt(date.year(), date.month(), last_day)
            .expect("valid last day of month");
        local_from_naive(last.and_hms_milli_opt(23, 59, 59, 999).expect("valid time"))
            .expect("local month end should exist")
    }

    #[must_use]
    pub fn begin_of_year(date: DateTime<Local>) -> DateTime<Local> {
        local_from_naive(
            NaiveDate::from_ymd_opt(date.year(), 1, 1)
                .expect("valid date")
                .and_hms_opt(0, 0, 0)
                .expect("valid time"),
        )
        .expect("local year start should exist")
    }

    #[must_use]
    pub fn end_of_year(date: DateTime<Local>) -> DateTime<Local> {
        local_from_naive(
            NaiveDate::from_ymd_opt(date.year(), 12, 31)
                .expect("valid date")
                .and_hms_milli_opt(23, 59, 59, 999)
                .expect("valid time"),
        )
        .expect("local year end should exist")
    }

    #[must_use]
    pub fn offset(date: DateTime<Local>, unit: DateUnit, offset: i64) -> DateTime<Local> {
        match unit {
            DateUnit::MS => date + Duration::milliseconds(offset),
            DateUnit::SECOND => date + Duration::seconds(offset),
            DateUnit::MINUTE => date + Duration::minutes(offset),
            DateUnit::HOUR => date + Duration::hours(offset),
            DateUnit::DAY => date + Duration::days(offset),
            DateUnit::WEEK => date + Duration::weeks(offset),
            DateUnit::MONTH => add_months(date, offset),
            DateUnit::YEAR => add_months(date, offset * 12),
        }
    }

    #[must_use]
    pub fn offset_day(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
        Self::offset(date, DateUnit::DAY, offset)
    }

    #[must_use]
    pub fn offset_hour(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
        Self::offset(date, DateUnit::HOUR, offset)
    }

    #[must_use]
    pub fn offset_minute(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
        Self::offset(date, DateUnit::MINUTE, offset)
    }

    #[must_use]
    pub fn yesterday() -> DateTime<Local> {
        Self::offset_day(Self::date(), -1)
    }

    #[must_use]
    pub fn tomorrow() -> DateTime<Local> {
        Self::offset_day(Self::date(), 1)
    }

    #[must_use]
    pub fn last_week() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::WEEK, -1)
    }

    #[must_use]
    pub fn next_week() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::WEEK, 1)
    }

    #[must_use]
    pub fn last_month() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::MONTH, -1)
    }

    #[must_use]
    pub fn next_month() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::MONTH, 1)
    }

    #[must_use]
    pub fn between(start: DateTime<Local>, end: DateTime<Local>, unit: DateUnit) -> i64 {
        let sign = if end >= start { 1 } else { -1 };
        let (start, end) = if end >= start {
            (start, end)
        } else {
            (end, start)
        };
        let duration = end.signed_duration_since(start);

        let value = match unit {
            DateUnit::MS => duration.num_milliseconds(),
            DateUnit::SECOND => duration.num_seconds(),
            DateUnit::MINUTE => duration.num_minutes(),
            DateUnit::HOUR => duration.num_hours(),
            DateUnit::DAY => duration.num_days(),
            DateUnit::WEEK => duration.num_days() / 7,
            DateUnit::MONTH => months_between(start, end),
            DateUnit::YEAR => months_between(start, end) / 12,
        };

        value * sign
    }

    #[must_use]
    pub fn format_between(between_ms: i64, level: Level) -> String {
        let mut millis = between_ms.abs();
        let days = millis / 86_400_000;
        millis %= 86_400_000;
        let hours = millis / 3_600_000;
        millis %= 3_600_000;
        let minutes = millis / 60_000;
        millis %= 60_000;
        let seconds = millis / 1_000;
        millis %= 1_000;

        let mut parts = Vec::new();
        match level {
            Level::YEAR | Level::MONTH | Level::DAY => parts.push(format!("{days}d")),
            Level::HOUR => {
                parts.push(format!("{days}d"));
                parts.push(format!("{hours}h"));
            }
            Level::MINUTE => {
                parts.push(format!("{days}d"));
                parts.push(format!("{hours}h"));
                parts.push(format!("{minutes}m"));
            }
            Level::SECOND => {
                parts.push(format!("{days}d"));
                parts.push(format!("{hours}h"));
                parts.push(format!("{minutes}m"));
                parts.push(format!("{seconds}s"));
            }
            Level::MILLISECOND => {
                parts.push(format!("{days}d"));
                parts.push(format!("{hours}h"));
                parts.push(format!("{minutes}m"));
                parts.push(format!("{seconds}s"));
                parts.push(format!("{millis}ms"));
            }
        }

        let result = parts.join(" ");
        if between_ms < 0 {
            format!("-{result}")
        } else {
            result
        }
    }

    #[must_use]
    pub fn get_zodiac(month: u32, day: u32) -> &'static str {
        if !(1..=12).contains(&month) {
            return "";
        }

        let idx = if day < ZODIAC_DATE[(month - 1) as usize] {
            (month + 10) % 12
        } else {
            (month - 1) % 12
        };

        ZODIAC[idx as usize]
    }

    #[must_use]
    pub fn get_chinese_zodiac(year: i32) -> &'static str {
        CHINESE_ZODIAC[(year - 1900).rem_euclid(12) as usize]
    }

    #[must_use]
    pub fn age_of_now(birth_date_str: &str) -> Option<u32> {
        Self::parse(birth_date_str).map(Self::age)
    }

    #[must_use]
    pub fn age(birth_date: DateTime<Local>) -> u32 {
        let now = Self::date();
        let mut age = now.year() - birth_date.year();
        if (now.month(), now.day()) < (birth_date.month(), birth_date.day()) {
            age -= 1;
        }
        age.max(0) as u32
    }

    #[must_use]
    pub fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    #[must_use]
    pub fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => 0,
        }
    }

    #[must_use]
    pub fn range(start: DateTime<Local>, end: DateTime<Local>, unit: DateUnit) -> DateRange {
        DateRange {
            start,
            end,
            current: start,
            unit,
        }
    }

    #[must_use]
    pub fn range_contains(range1: DateRange, range2: DateRange) -> Vec<DateTime<Local>> {
        let start = range1.start.max(range2.start);
        let end = range1.end.min(range2.end);
        if start > end {
            return Vec::new();
        }
        Self::range_to_list(start, end, range1.unit.min_precision(range2.unit))
    }

    #[must_use]
    pub fn range_not_contains(range1: DateRange, range2: DateRange) -> Vec<DateTime<Local>> {
        let mut result = Vec::new();

        if range2.start < range1.start {
            result.extend(Self::range_to_list(
                range2.start,
                Self::offset(range1.start, range2.unit, -1),
                range2.unit,
            ));
        }

        if range2.end > range1.end {
            result.extend(Self::range_to_list(
                Self::offset(range1.end, range2.unit, 1),
                range2.end,
                range2.unit,
            ));
        }

        result
    }

    #[must_use]
    pub fn range_to_list(
        start: DateTime<Local>,
        end: DateTime<Local>,
        unit: DateUnit,
    ) -> Vec<DateTime<Local>> {
        if start > end {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current = start;
        while current <= end {
            result.push(current);
            current = Self::offset(current, unit, 1);
        }
        result
    }
}

impl DateUnit {
    fn rank(self) -> u8 {
        match self {
            DateUnit::MS => 0,
            DateUnit::SECOND => 1,
            DateUnit::MINUTE => 2,
            DateUnit::HOUR => 3,
            DateUnit::DAY => 4,
            DateUnit::WEEK => 5,
            DateUnit::MONTH => 6,
            DateUnit::YEAR => 7,
        }
    }

    fn min_precision(self, other: Self) -> Self {
        if self.rank() <= other.rank() {
            self
        } else {
            other
        }
    }
}

#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub current: DateTime<Local>,
    pub unit: DateUnit,
}

impl Iterator for DateRange {
    type Item = DateTime<Local>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.end {
            return None;
        }

        let current = self.current;
        self.current = DateUtil::offset(self.current, self.unit, 1);
        Some(current)
    }
}

fn local_timestamp(seconds: i64, nanos: u32) -> DateTime<Local> {
    match Local.timestamp_opt(seconds, nanos) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => Local
            .timestamp_opt(0, 0)
            .single()
            .unwrap_or_else(Local::now),
    }
}

fn local_from_naive(value: NaiveDateTime) -> Option<DateTime<Local>> {
    match Local.from_local_datetime(&value) {
        LocalResult::Single(value) => Some(value),
        LocalResult::Ambiguous(first, _) => Some(first),
        LocalResult::None => None,
    }
}

fn add_months(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
    let total_months = date.year() as i64 * 12 + date.month0() as i64 + offset;
    let new_year = total_months.div_euclid(12) as i32;
    let new_month0 = total_months.rem_euclid(12) as u32;
    let new_month = new_month0 + 1;
    let day = date.day().min(DateUtil::days_in_month(new_year, new_month));
    let naive = NaiveDate::from_ymd_opt(new_year, new_month, day)
        .expect("valid shifted month")
        .and_hms_nano_opt(
            date.hour(),
            date.minute(),
            date.second(),
            date.timestamp_subsec_nanos(),
        )
        .expect("valid shifted time");

    local_from_naive(naive).unwrap_or_else(|| date + Duration::days(offset * 30))
}

fn months_between(start: DateTime<Local>, end: DateTime<Local>) -> i64 {
    let mut months =
        (end.year() - start.year()) as i64 * 12 + end.month() as i64 - start.month() as i64;
    if (end.day(), end.time()) < (start.day(), start.time()) {
        months -= 1;
    }
    months
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, TimeZone};

    use super::{DateUnit, DateUtil, Level};

    #[test]
    fn parse_common_formats() {
        assert!(DateUtil::parse("2024-02-29").is_some());
        assert!(DateUtil::parse("2024-02-29 12:30:45").is_some());
        assert!(DateUtil::parse("2024-02-29T12:30:45Z").is_some());
    }

    #[test]
    fn month_offset_clamps_to_valid_day() {
        let date = chrono::Local
            .with_ymd_and_hms(2024, 3, 31, 10, 0, 0)
            .single()
            .unwrap();
        let shifted = DateUtil::offset(date, DateUnit::MONTH, -1);
        assert_eq!(shifted.month(), 2);
        assert_eq!(shifted.day(), 29);
    }

    #[test]
    fn between_and_range_work() {
        let start = DateUtil::parse("2024-01-01").unwrap();
        let end = DateUtil::parse("2024-01-03").unwrap();

        assert_eq!(DateUtil::between(start, end, DateUnit::DAY), 2);
        assert_eq!(DateUtil::range_to_list(start, end, DateUnit::DAY).len(), 3);
    }

    #[test]
    fn format_between_uses_requested_precision() {
        assert_eq!(
            DateUtil::format_between(90_061_005, Level::MILLISECOND),
            "1d 1h 1m 1s 5ms"
        );
    }
}
