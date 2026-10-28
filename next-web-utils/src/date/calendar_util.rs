use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Weekday};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarViewType {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub all_day: bool,
    pub description: Option<String>,
    pub location: Option<String>,
    pub color: Option<String>,
    pub repeated: bool,
    pub repeat_rule: Option<String>,
    pub reminder: Option<i64>,
    pub tags: Vec<String>,
}

impl CalendarEvent {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        start_time: DateTime<Local>,
        end_time: DateTime<Local>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            start_time,
            end_time,
            all_day: false,
            description: None,
            location: None,
            color: None,
            repeated: false,
            repeat_rule: None,
            reminder: None,
            tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn occurs_on(&self, date: NaiveDate) -> bool {
        self.start_time.date_naive() <= date && self.end_time.date_naive() >= date
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthCalendar {
    pub year: i32,
    pub month: u32,
    pub weeks: Vec<Vec<DayInfo>>,
    pub total_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayInfo {
    pub date: NaiveDate,
    pub is_today: bool,
    pub is_current_month: bool,
    pub is_weekend: bool,
    pub is_holiday: bool,
    pub holiday_name: Option<String>,
    pub lunar_date: Option<String>,
    pub solar_term: Option<String>,
    pub events: Vec<CalendarEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekInfo {
    pub year: i32,
    pub week_number: u32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub days: Vec<DayInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolidayInfo {
    pub name: String,
    pub date: NaiveDate,
    pub is_work_day: bool,
}

pub struct CalendarUtil;

impl CalendarUtil {
    #[must_use]
    pub fn month_name(month: u32) -> &'static str {
        match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Invalid Month",
        }
    }

    #[must_use]
    pub fn month_name_en(month: u32) -> &'static str {
        Self::month_name(month)
    }

    #[must_use]
    pub fn month_name_zh(month: u32) -> &'static str {
        match month {
            1 => "一月",
            2 => "二月",
            3 => "三月",
            4 => "四月",
            5 => "五月",
            6 => "六月",
            7 => "七月",
            8 => "八月",
            9 => "九月",
            10 => "十月",
            11 => "十一月",
            12 => "十二月",
            _ => "无效月份",
        }
    }

    #[must_use]
    pub fn weekday_name(weekday: &Weekday) -> &'static str {
        match weekday {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
    }

    #[must_use]
    pub fn weekday_name_en(weekday: &Weekday) -> &'static str {
        Self::weekday_name(weekday)
    }

    #[must_use]
    pub fn weekday_short_name(weekday: &Weekday) -> &'static str {
        match weekday {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        }
    }

    #[must_use]
    pub fn weekday_short_name_en(weekday: &Weekday) -> &'static str {
        Self::weekday_short_name(weekday)
    }

    #[must_use]
    pub fn weekday_short_name_zh(weekday: &Weekday) -> &'static str {
        match weekday {
            Weekday::Mon => "一",
            Weekday::Tue => "二",
            Weekday::Wed => "三",
            Weekday::Thu => "四",
            Weekday::Fri => "五",
            Weekday::Sat => "六",
            Weekday::Sun => "日",
        }
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
    pub fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    #[must_use]
    pub fn quarters_in_year() -> u32 {
        4
    }

    #[must_use]
    pub fn months_in_quarter(quarter: u32) -> Vec<u32> {
        match quarter {
            1 => vec![1, 2, 3],
            2 => vec![4, 5, 6],
            3 => vec![7, 8, 9],
            4 => vec![10, 11, 12],
            _ => Vec::new(),
        }
    }

    #[must_use]
    pub fn quarter_of_month(month: u32) -> Option<u32> {
        (1..=12).contains(&month).then_some((month - 1) / 3 + 1)
    }

    #[must_use]
    pub fn day_of_year(date: NaiveDate) -> u32 {
        date.ordinal()
    }

    #[must_use]
    pub fn day_of_week(date: NaiveDate) -> u32 {
        date.weekday().number_from_monday()
    }

    #[must_use]
    pub fn week_of_year(date: NaiveDate) -> u32 {
        date.iso_week().week()
    }

    #[must_use]
    pub fn first_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, month, 1)
    }

    #[must_use]
    pub fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, month, Self::days_in_month(year, month))
    }

    #[must_use]
    pub fn first_day_of_quarter(year: i32, quarter: u32) -> Option<NaiveDate> {
        let month = Self::months_in_quarter(quarter).first().copied()?;
        NaiveDate::from_ymd_opt(year, month, 1)
    }

    #[must_use]
    pub fn last_day_of_quarter(year: i32, quarter: u32) -> Option<NaiveDate> {
        let month = Self::months_in_quarter(quarter).last().copied()?;
        Self::last_day_of_month(year, month)
    }

    #[must_use]
    pub fn first_day_of_year(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 1, 1)
    }

    #[must_use]
    pub fn last_day_of_year(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 12, 31)
    }

    #[must_use]
    pub fn is_weekend(date: NaiveDate) -> bool {
        matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }

    #[must_use]
    pub fn is_weekday(date: NaiveDate) -> bool {
        !Self::is_weekend(date)
    }

    #[must_use]
    pub fn get_month_grid(year: i32, month: u32) -> Vec<Vec<Option<u32>>> {
        let Some(first_day) = Self::first_day_of_month(year, month) else {
            return Vec::new();
        };
        let days_in_month = Self::days_in_month(year, month);
        let mut grid = Vec::new();
        let mut week = Vec::new();

        for _ in 0..first_day.weekday().num_days_from_monday() {
            week.push(None);
        }

        for day in 1..=days_in_month {
            week.push(Some(day));
            if week.len() == 7 {
                grid.push(week);
                week = Vec::new();
            }
        }

        if !week.is_empty() {
            while week.len() < 7 {
                week.push(None);
            }
            grid.push(week);
        }

        grid
    }

    #[must_use]
    pub fn month_calendar(
        year: i32,
        month: u32,
        events: &[CalendarEvent],
        holidays: &[HolidayInfo],
    ) -> Option<MonthCalendar> {
        let first = Self::first_day_of_month(year, month)?;
        let grid = Self::get_month_grid(year, month);
        let today = Self::today();
        let mut weeks = Vec::new();

        let mut cursor = first - Duration::days(first.weekday().num_days_from_monday() as i64);
        for _ in 0..grid.len() {
            let mut week = Vec::new();
            for _ in 0..7 {
                week.push(Self::day_info(cursor, month, today, events, holidays));
                cursor += Duration::days(1);
            }
            weeks.push(week);
        }

        Some(MonthCalendar {
            year,
            month,
            weeks,
            total_days: Self::days_in_month(year, month),
        })
    }

    #[must_use]
    pub fn week_info(
        date: NaiveDate,
        events: &[CalendarEvent],
        holidays: &[HolidayInfo],
    ) -> WeekInfo {
        let start_date = date - Duration::days(date.weekday().num_days_from_monday() as i64);
        let end_date = start_date + Duration::days(6);
        let today = Self::today();
        let days = (0..7)
            .map(|offset| {
                Self::day_info(
                    start_date + Duration::days(offset),
                    date.month(),
                    today,
                    events,
                    holidays,
                )
            })
            .collect();

        WeekInfo {
            year: date.iso_week().year(),
            week_number: date.iso_week().week(),
            start_date,
            end_date,
            days,
        }
    }

    #[must_use]
    pub fn days_between(start: NaiveDate, end: NaiveDate) -> i64 {
        (end - start).num_days()
    }

    #[must_use]
    pub fn weekdays_between(start: NaiveDate, end: NaiveDate) -> i64 {
        if start > end {
            return -Self::weekdays_between(end, start);
        }

        let mut count = 0;
        let mut current = start;
        while current <= end {
            if Self::is_weekday(current) {
                count += 1;
            }
            current += Duration::days(1);
        }
        count
    }

    #[must_use]
    pub fn is_same_month(date1: NaiveDate, date2: NaiveDate) -> bool {
        date1.year() == date2.year() && date1.month() == date2.month()
    }

    #[must_use]
    pub fn is_same_year(date1: NaiveDate, date2: NaiveDate) -> bool {
        date1.year() == date2.year()
    }

    #[must_use]
    pub fn format_date(date: NaiveDate, format: &str) -> String {
        date.format(format).to_string()
    }

    #[must_use]
    pub fn today() -> NaiveDate {
        Local::now().date_naive()
    }

    #[must_use]
    pub fn first_day_of_current_week() -> NaiveDate {
        let today = Self::today();
        today - Duration::days(today.weekday().num_days_from_monday() as i64)
    }

    #[must_use]
    pub fn same_day_last_month(date: NaiveDate) -> Option<NaiveDate> {
        let (year, month) = if date.month() == 1 {
            (date.year() - 1, 12)
        } else {
            (date.year(), date.month() - 1)
        };
        let day = date.day().min(Self::days_in_month(year, month));
        NaiveDate::from_ymd_opt(year, month, day)
    }

    #[must_use]
    pub fn same_day_next_month(date: NaiveDate) -> Option<NaiveDate> {
        let (year, month) = if date.month() == 12 {
            (date.year() + 1, 1)
        } else {
            (date.year(), date.month() + 1)
        };
        let day = date.day().min(Self::days_in_month(year, month));
        NaiveDate::from_ymd_opt(year, month, day)
    }

    #[must_use]
    fn day_info(
        date: NaiveDate,
        current_month: u32,
        today: NaiveDate,
        events: &[CalendarEvent],
        holidays: &[HolidayInfo],
    ) -> DayInfo {
        let holiday = holidays.iter().find(|holiday| holiday.date == date);
        DayInfo {
            date,
            is_today: date == today,
            is_current_month: date.month() == current_month,
            is_weekend: Self::is_weekend(date),
            is_holiday: holiday.is_some_and(|holiday| !holiday.is_work_day),
            holiday_name: holiday.map(|holiday| holiday.name.clone()),
            lunar_date: None,
            solar_term: None,
            events: events
                .iter()
                .filter(|event| event.occurs_on(date))
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LunarCalendar;

impl LunarCalendar {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn get_lunar_date(&self, date: NaiveDate) -> String {
        date.format("%Y-%m-%d").to_string()
    }

    #[must_use]
    pub fn get_solar_term(&self, _date: NaiveDate) -> Option<String> {
        None
    }

    #[must_use]
    pub fn get_zodiac_animal(&self, year: i32) -> &'static str {
        const ANIMALS: [&str; 12] = [
            "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Goat", "Monkey",
            "Rooster", "Dog", "Pig",
        ];
        ANIMALS[(year - 4).rem_euclid(12) as usize]
    }

    #[must_use]
    pub fn get_traditional_festival(&self, _date: NaiveDate) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate};

    use super::CalendarUtil;

    #[test]
    fn month_grid_has_calendar_weeks() {
        let grid = CalendarUtil::get_month_grid(2024, 2);
        assert_eq!(grid.len(), 5);
        assert_eq!(grid[0][3], Some(1));
        assert_eq!(grid[4][3], Some(29));
    }

    #[test]
    fn month_arithmetic_clamps_end_of_month() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 31).unwrap();
        assert_eq!(
            CalendarUtil::same_day_last_month(date),
            NaiveDate::from_ymd_opt(2024, 2, 29)
        );
    }

    #[test]
    fn weekdays_between_counts_inclusive_weekdays() {
        let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();

        assert_eq!(CalendarUtil::weekdays_between(monday, sunday), 5);
    }

    #[test]
    fn week_info_uses_monday_start() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
        let week = CalendarUtil::week_info(date, &[], &[]);

        assert_eq!(week.start_date.weekday().number_from_monday(), 1);
        assert_eq!(week.end_date.weekday().number_from_monday(), 7);
    }
}
