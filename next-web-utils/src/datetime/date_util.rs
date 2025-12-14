use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use once_cell::sync::Lazy;

/// 日期单位枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateUnit {
    MS,     // 毫秒
    SECOND, // 秒
    MINUTE, // 分
    HOUR,   // 小时
    DAY,    // 天
    WEEK,   // 周
    MONTH,  // 月
    YEAR,   // 年
}

/// 时间精度
pub enum Level {
    YEAR,
    MONTH,
    DAY,
    HOUR,
    MINUTE,
    SECOND,
    MILLISECOND,
}

/// 中国属相
pub const CHINESE_ZODIAC: [&str; 12] = [
    "鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪",
];

/// 星座
pub const ZODIAC: [&str; 12] = [
    "水瓶座",
    "双鱼座",
    "白羊座",
    "金牛座",
    "双子座",
    "巨蟹座",
    "狮子座",
    "处女座",
    "天秤座",
    "天蝎座",
    "射手座",
    "摩羯座",
];
pub const ZODIAC_DATE: [u32; 12] = [20, 19, 21, 20, 21, 22, 23, 23, 23, 24, 23, 22];

static DATE_FORMATS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y.%m.%d %H:%M:%S",
        "%Y年%m月%d日 %H时%M分%S秒",
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y.%m.%d",
        "%H:%M:%S",
        "%H时%M分%S秒",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d %H:%M:%S.%3f",
        "%Y%m%d%H%M%S",
        "%Y%m%d%H%M%S%.3f",
        "%Y%m%d",
        "%a, %d %b %Y %H:%M:%S %Z",
        "%a %b %d %H:%M:%S %Z %Y",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S.%3fZ",
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%dT%H:%M:%S.%3f%z",
    ]
});

pub struct DateUtil;

impl DateUtil {
    /// 获取当前日期时间
    pub fn date() -> DateTime<Local> {
        Local::now()
    }

    /// 从时间戳创建日期（毫秒）
    pub fn date_ms(timestamp_ms: i64) -> DateTime<Local> {
        let seconds = timestamp_ms / 1000;
        let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;
        Local.timestamp_opt(seconds, nanos).unwrap()
    }

    /// 当前时间字符串，格式：yyyy-MM-dd HH:mm:ss
    pub fn now() -> String {
        Self::format(Self::date(), "%Y-%m-%d %H:%M:%S")
    }

    /// 当前日期字符串，格式：yyyy-MM-dd
    pub fn today() -> String {
        Self::format(Self::date(), "%Y-%m-%d")
    }

    /// 自动识别常用格式解析日期字符串
    pub fn parse(date_str: &str) -> Option<DateTime<Local>> {
        for format in DATE_FORMATS.iter() {
            if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, format) {
                return Some(Local.from_local_datetime(&dt).unwrap());
            }

            // 尝试只解析日期部分
            if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
                let dt = date.and_hms_opt(0, 0, 0).unwrap();
                return Some(Local.from_local_datetime(&dt).unwrap());
            }
        }
        None
    }

    /// 使用指定格式解析日期字符串
    pub fn parse_with_format(date_str: &str, format: &str) -> Option<DateTime<Local>> {
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, format) {
            return Some(Local.from_local_datetime(&dt).unwrap());
        }

        if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
            let dt = date.and_hms_opt(0, 0, 0).unwrap();
            return Some(Local.from_local_datetime(&dt).unwrap());
        }

        None
    }

    /// 格式化日期
    pub fn format(date: DateTime<Local>, fmt: &str) -> String {
        date.format(fmt).to_string()
    }

    /// 格式化为标准日期 yyyy-MM-dd
    pub fn format_date(date: DateTime<Local>) -> String {
        Self::format(date, "%Y-%m-%d")
    }

    /// 格式化为标准日期时间 yyyy-MM-dd HH:mm:ss
    pub fn format_datetime(date: DateTime<Local>) -> String {
        Self::format(date, "%Y-%m-%d %H:%M:%S")
    }

    /// 格式化为标准时间 HH:mm:ss
    pub fn format_time(date: DateTime<Local>) -> String {
        Self::format(date, "%H:%M:%S")
    }

    /// 获取年份
    pub fn year(date: DateTime<Local>) -> i32 {
        date.year()
    }

    /// 获取月份 (1-12)
    pub fn month(date: DateTime<Local>) -> u32 {
        date.month()
    }

    /// 获取日期在月中的天数 (1-31)
    pub fn day(date: DateTime<Local>) -> u32 {
        date.day()
    }

    /// 获取小时 (0-23)
    pub fn hour(date: DateTime<Local>) -> u32 {
        date.hour()
    }

    /// 获取分钟 (0-59)
    pub fn minute(date: DateTime<Local>) -> u32 {
        date.minute()
    }

    /// 获取秒 (0-59)
    pub fn second(date: DateTime<Local>) -> u32 {
        date.second()
    }

    /// 获取星期几 (1-7, 周一到周日)
    pub fn day_of_week(date: DateTime<Local>) -> u32 {
        date.weekday().number_from_monday()
    }

    /// 获取一天的开始时间
    pub fn begin_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
    }

    /// 获取一天的结束时间
    pub fn end_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
    }

    /// 获取一月的开始时间
    pub fn begin_of_month(date: DateTime<Local>) -> DateTime<Local> {
        let naive_date = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
        naive_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
    }

    /// 获取一月的结束时间
    pub fn end_of_month(date: DateTime<Local>) -> DateTime<Local> {
        let last_day = if date.month() == 12 {
            NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
        }
        .pred_opt()
        .unwrap();

        last_day
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
    }

    /// 日期偏移
    pub fn offset(date: DateTime<Local>, unit: DateUnit, offset: i64) -> DateTime<Local> {
        match unit {
            DateUnit::MS => date + Duration::milliseconds(offset),
            DateUnit::SECOND => date + Duration::seconds(offset),
            DateUnit::MINUTE => date + Duration::minutes(offset),
            DateUnit::HOUR => date + Duration::hours(offset),
            DateUnit::DAY => date + Duration::days(offset),
            DateUnit::WEEK => date + Duration::weeks(offset),
            DateUnit::MONTH => {
                let month = date.month() as i32 + offset as i32;
                let year_offset = (month - 1) / 12;
                let new_month = ((month - 1) % 12 + 1) as u32;
                let new_year = date.year() + year_offset;

                let new_day = date.day().min(Self::days_in_month(new_year, new_month));

                NaiveDate::from_ymd_opt(new_year, new_month, new_day)
                    .unwrap()
                    .and_hms_opt(date.hour(), date.minute(), date.second())
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap()
            }
            DateUnit::YEAR => {
                let new_year = date.year() + offset as i32;
                let naive_date = NaiveDate::from_ymd_opt(
                    new_year,
                    date.month(),
                    date.day().min(Self::days_in_month(new_year, date.month())),
                )
                .unwrap();

                naive_date
                    .and_hms_opt(date.hour(), date.minute(), date.second())
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap()
            }
        }
    }

    /// 获取月份天数
    fn days_in_month(year: i32, month: u32) -> u32 {
        NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
    }

    /// 偏移天数
    pub fn offset_day(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
        Self::offset(date, DateUnit::DAY, offset)
    }

    /// 偏移小时
    pub fn offset_hour(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
        Self::offset(date, DateUnit::HOUR, offset)
    }

    /// 偏移分钟
    pub fn offset_minute(date: DateTime<Local>, offset: i64) -> DateTime<Local> {
        Self::offset(date, DateUnit::MINUTE, offset)
    }

    /// 昨天
    pub fn yesterday() -> DateTime<Local> {
        Self::offset_day(Self::date(), -1)
    }

    /// 明天
    pub fn tomorrow() -> DateTime<Local> {
        Self::offset_day(Self::date(), 1)
    }

    /// 上周
    pub fn last_week() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::WEEK, -1)
    }

    /// 下周
    pub fn next_week() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::WEEK, 1)
    }

    /// 上个月
    pub fn last_month() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::MONTH, -1)
    }

    /// 下个月
    pub fn next_month() -> DateTime<Local> {
        Self::offset(Self::date(), DateUnit::MONTH, 1)
    }

    /// 计算两个日期之间的差值
    pub fn between(start: DateTime<Local>, end: DateTime<Local>, unit: DateUnit) -> i64 {
        let duration = end.signed_duration_since(start);

        match unit {
            DateUnit::MS => duration.num_milliseconds(),
            DateUnit::SECOND => duration.num_seconds(),
            DateUnit::MINUTE => duration.num_minutes(),
            DateUnit::HOUR => duration.num_hours(),
            DateUnit::DAY => duration.num_days(),
            DateUnit::WEEK => duration.num_days() / 7,
            DateUnit::MONTH => {
                let (y1, m1, d1) = (start.year(), start.month(), start.day());
                let (y2, m2, d2) = (end.year(), end.month(), end.day());

                let months = (y2 - y1) * 12 + (m2 as i32 - m1 as i32);

                let months = if d2 < d1 { months - 1 } else { months };
                months as i64
            }
            DateUnit::YEAR => {
                let (y1, m1, d1) = (start.year(), start.month(), start.day());
                let (y2, m2, d2) = (end.year(), end.month(), end.day());

                let years = y2 - y1;

                let years = if m2 < m1 || (m2 == m1 && d2 < d1) {
                    years - 1
                } else {
                    years
                };

                years as i64
            }
        }
    }

    /// 格式化时间差
    pub fn format_between(between_ms: i64, level: Level) -> String {
        let total_seconds = between_ms / 1000;
        let days = total_seconds / (24 * 3600);
        let hours = (total_seconds % (24 * 3600)) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let ms = between_ms % 1000;

        match level {
            Level::DAY => format!("{}天", days),
            Level::HOUR => format!("{}天{}小时", days, hours),
            Level::MINUTE => format!("{}天{}小时{}分", days, hours, minutes),
            Level::SECOND => format!("{}天{}小时{}分{}秒", days, hours, minutes, seconds),
            Level::MILLISECOND => format!(
                "{}天{}小时{}分{}秒{}毫秒",
                days, hours, minutes, seconds, ms
            ),
            _ => format!("{}天", days),
        }
    }

    /// 获取星座
    pub fn get_zodiac(month: u32, day: u32) -> &'static str {
        let idx = if day < ZODIAC_DATE[month as usize - 1] {
            (month - 2 + 12) % 12
        } else {
            (month - 1) % 12
        };

        ZODIAC[idx as usize]
    }

    /// 获取生肖
    pub fn get_chinese_zodiac(year: i32) -> &'static str {
        CHINESE_ZODIAC[((year - 1900) % 12) as usize]
    }

    /// 计算年龄
    pub fn age_of_now(birth_date_str: &str) -> Option<u32> {
        if let Some(birth_date) = Self::parse(birth_date_str) {
            let now = Self::date();
            let mut age = now.year() - birth_date.year();

            if now.month() < birth_date.month()
                || (now.month() == birth_date.month() && now.day() < birth_date.day())
            {
                age -= 1;
            }

            Some(age as u32)
        } else {
            None
        }
    }

    /// 判断是否闰年
    pub fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    /// 创建日期范围迭代器
    pub fn range(start: DateTime<Local>, end: DateTime<Local>, unit: DateUnit) -> DateRange {
        DateRange {
            start,
            end,
            current: start,
            unit,
        }
    }

    /// 计算两个范围的交集
    pub fn range_contains(range1: DateRange, range2: DateRange) -> Vec<DateTime<Local>> {
        let start = if range1.start > range2.start {
            range1.start
        } else {
            range2.start
        };
        let end = if range1.end < range2.end {
            range1.end
        } else {
            range2.end
        };

        if start > end {
            return vec![];
        }

        // 使用较小的时间单位
        let unit = match (range1.unit, range2.unit) {
            (DateUnit::MS, _) | (_, DateUnit::MS) => DateUnit::MS,
            (DateUnit::SECOND, _) | (_, DateUnit::SECOND) => DateUnit::SECOND,
            (DateUnit::MINUTE, _) | (_, DateUnit::MINUTE) => DateUnit::MINUTE,
            (DateUnit::HOUR, _) | (_, DateUnit::HOUR) => DateUnit::HOUR,
            (DateUnit::DAY, _) | (_, DateUnit::DAY) => DateUnit::DAY,
            (DateUnit::WEEK, _) | (_, DateUnit::WEEK) => DateUnit::WEEK,
            (DateUnit::MONTH, _) | (_, DateUnit::MONTH) => DateUnit::MONTH,
            (DateUnit::YEAR, DateUnit::YEAR) => DateUnit::YEAR,
        };

        Self::range_to_list(start, end, unit)
    }

    /// 计算两个范围的差集 (range2 中有但 range1 中没有的)
    pub fn range_not_contains(range1: DateRange, range2: DateRange) -> Vec<DateTime<Local>> {
        let mut result = vec![];

        // 使用范围2的单位
        let unit = range2.unit;

        // 如果范围2的起始日期早于范围1的起始日期，添加差集部分
        if range2.start < range1.start {
            result.append(&mut Self::range_to_list(
                range2.start,
                range1.start,
                unit.clone(),
            ));
        }

        // 如果范围2的结束日期晚于范围1的结束日期，添加差集部分
        if range2.end > range1.end {
            result.append(&mut Self::range_to_list(range1.end, range2.end, unit));
        }

        result
    }

    /// 将日期范围转换为列表
    pub fn range_to_list(
        start: DateTime<Local>,
        end: DateTime<Local>,
        unit: DateUnit,
    ) -> Vec<DateTime<Local>> {
        let mut result = vec![];
        let mut current = start;

        while current <= end {
            result.push(current);
            current = Self::offset(current, unit.clone(), 1);
        }

        result
    }
}

// 为日期范围实现迭代器特性
impl Iterator for DateRange {
    type Item = DateTime<Local>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.end {
            return None;
        }

        let result = self.current;
        self.current = DateUtil::offset(self.current, self.unit.clone(), 1);

        Some(result)
    }
}

/// 创建日期范围 - 返回起始日期和结束日期
pub struct DateRange {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub current: DateTime<Local>,
    pub unit: DateUnit,
}
