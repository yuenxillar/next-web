use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc, FixedOffset};

/// 日期时间工具，提供全面的日期时间操作
pub struct DateTimeUtil;

impl DateTimeUtil {
    /// 获取当前时间，格式化为 "YYYY-MM-DD HH:MM:SS.mmm"
    pub fn now() -> String {
        let now: DateTime<Local> = Local::now();
        now.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }

    /// 获取当前时间戳（毫秒）
    pub fn timestamp() -> i64 {
        Local::now().timestamp_millis()
    }

    /// 获取当前日期，格式化为 "YYYY-MM-DD"
    pub fn date() -> String {
        Local::now().format("%Y-%m-%d").to_string()
    }

    /// 获取当前时间，格式化为 "HH:MM:SS"
    pub fn time() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    /// 自定义格式化当前时间
    pub fn format_now(fmt: &str) -> String {
        Local::now().format(fmt).to_string()
    }

    /// 将时间戳转换为日期时间字符串
    pub fn from_timestamp(timestamp: i64, fmt: &str) -> String {
        let dt = Local.timestamp_opt(timestamp, 0).unwrap();
        dt.format(fmt).to_string()
    }

    /// 将字符串解析为 NaiveDateTime
    pub fn parse(datetime_str: &str, fmt: &str) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(datetime_str, fmt).ok()
    }

    /// 检查是否为同一天
    pub fn is_same_day(dt1: &NaiveDateTime, dt2: &NaiveDateTime) -> bool {
        dt1.date() == dt2.date()
    }

    /// 添加天数
    pub fn add_days(dt: &NaiveDateTime, days: i64) -> NaiveDateTime {
        *dt + Duration::days(days)
    }

    /// 添加小时
    pub fn add_hours(dt: &NaiveDateTime, hours: i64) -> NaiveDateTime {
        *dt + Duration::hours(hours)
    }

    /// 添加分钟
    pub fn add_minutes(dt: &NaiveDateTime, minutes: i64) -> NaiveDateTime {
        *dt + Duration::minutes(minutes)
    }

    /// 计算两个日期之间的天数差
    pub fn days_between(dt1: &NaiveDateTime, dt2: &NaiveDateTime) -> i64 {
        let duration = *dt2 - *dt1;
        duration.num_days()
    }

    /// 获取当月第一天
    pub fn first_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, month, 1)
    }

    /// 获取当月最后一天
    pub fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
        let first_of_next_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        };
        
        first_of_next_month.map(|d| d.pred_opt().unwrap())
    }

    /// 检查是否为工作日（非周末）
    pub fn is_weekday(dt: &NaiveDateTime) -> bool {
        let weekday = dt.weekday().number_from_monday();
        weekday >= 1 && weekday <= 5
    }

    /// 检查是否为周末
    pub fn is_weekend(dt: &NaiveDateTime) -> bool {
        let weekday = dt.weekday().number_from_monday();
        weekday == 6 || weekday == 7
    }

    /// 转换为 UTC 时间
    pub fn to_utc(dt: DateTime<Local>) -> DateTime<Utc> {
        dt.with_timezone(&Utc)
    }

    /// 转换时区
    pub fn change_timezone(dt: &DateTime<Utc>, hours: i32) -> DateTime<FixedOffset> {
        let offset = FixedOffset::east_opt(hours * 3600).unwrap();
        dt.with_timezone(&offset)
    }

    /// 获取季度
    pub fn get_quarter(dt: &NaiveDateTime) -> u32 {
        (dt.month() - 1) / 3 + 1
    }

    /// 年龄计算
    pub fn calculate_age(birth_date: &NaiveDate) -> u32 {
        let today = Local::now().naive_local().date();
        let age = today.year() - birth_date.year();
        
        if today.month() < birth_date.month() || 
           (today.month() == birth_date.month() && today.day() < birth_date.day()) {
            return (age - 1) as u32;
        }
        
        age as u32
    }
}