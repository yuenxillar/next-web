use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};

/// 本地日期时间工具
///
/// 提供一系列用于获取和格式化当前本地日期时间的静态方法。
///
/// # Local Date and Time Utility
///
/// Provides a set of static methods for retrieving and formatting the current local date and time.
pub struct LocalDateTime;

impl LocalDateTime {
    /// 获取当前时间的 `DateTime<Local>` 实例
    ///
    /// # Returns
    ///
    /// 当前的本地日期时间。
    ///
    /// # Get Current Local DateTime
    ///
    /// # Returns
    ///
    /// The current local date and time.
    fn _now() -> DateTime<Local> {
        Local::now()
    }

    /// 获取当前日期时间的字符串表示（格式：YYYY-MM-DD HH:MM:SS.fff）
    ///
    /// # Returns
    ///
    /// 格式化后的时间字符串，精确到毫秒。
    ///
    /// # Get Current Date and Time as String
    ///
    /// Formats the current date and time as "YYYY-MM-DD HH:MM:SS.fff".
    ///
    /// # Returns
    ///
    /// Formatted time string with millisecond precision.
    pub fn now() -> String {
        Self::_now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }

    /// 获取当前时间的 Unix 时间戳（秒）
    ///
    /// # Returns
    ///
    /// 从 Unix 纪元（1970-01-01 00:00:00 UTC）到现在的秒数。
    ///
    /// # Get Current Unix Timestamp (Seconds)
    ///
    /// # Returns
    ///
    /// Number of seconds since the Unix epoch (1970-01-01 00:00:00 UTC).
    pub fn timestamp() -> i64 {
        Self::_now().timestamp()
    }

    /// 获取当前日期的字符串表示（格式：YYYY-MM-DD）
    ///
    /// # Returns
    ///
    /// 格式化的日期字符串。
    ///
    /// # Get Current Date as String
    ///
    /// Formats the current date as "YYYY-MM-DD".
    ///
    /// # Returns
    ///
    /// Formatted date string.
    pub fn date() -> String {
        Self::_now().format("%Y-%m-%d").to_string()
    }

    /// 获取用于保存文件的路径格式日期（格式：YYYY/MM/DD）
    ///
    /// 常用于日志、备份等需要按日期分层存储的场景。
    ///
    /// # Returns
    ///
    /// 分层的日期路径字符串。
    ///
    /// # Get Date for Save Path
    ///
    /// Formats the current date as "YYYY/MM/DD", suitable for directory paths.
    /// Commonly used for organizing logs or backups by date.
    ///
    /// # Returns
    ///
    /// Hierarchical date path string.
    pub fn save_path() -> String {
        Self::_now().format("%Y/%m/%d").to_string()
    }

    /// 使用自定义格式化字符串获取当前时间的字符串表示
    ///
    /// # Parameters
    ///
    /// * `format` - 符合 `chrono` 库规范的格式字符串。
    ///
    /// # Returns
    ///
    /// 根据指定格式生成的时间字符串。
    ///
    /// # Get Formatted Date and Time with Custom Format
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

    /// 获取当前时间的毫秒级时间戳
    ///
    /// # Returns
    ///
    /// 从 Unix 纪元到现在的毫秒数。
    ///
    /// # Get Millisecond Timestamp
    ///
    /// # Returns
    ///
    /// Number of milliseconds since the Unix epoch.
    pub fn timestamp_millis() -> i64 {
        Self::_now().timestamp_millis()
    }

    /// 获取当前时间的微秒级时间戳
    ///
    /// # Returns
    ///
    /// 从 Unix 纪元到现在的微秒数。
    ///
    /// # Get Microsecond Timestamp
    ///
    /// # Returns
    ///
    /// Number of microseconds since the Unix epoch.
    pub fn timestamp_micros() -> i64 {
        Self::_now().timestamp_micros()
    }

    /// 获取当前是星期几（中文）
    ///
    /// # Returns
    ///
    /// 星期一至星期日的中文字符串。
    ///
    /// # Get Day of Week in Chinese
    ///
    /// # Returns
    ///
    /// Chinese string representing the day of week (Monday to Sunday).
    pub fn day_of_week_zh() -> String {
        let weekday = Self::_now().weekday();
        match weekday {
            chrono::Weekday::Mon => "星期一",
            chrono::Weekday::Tue => "星期二",
            chrono::Weekday::Wed => "星期三",
            chrono::Weekday::Thu => "星期四",
            chrono::Weekday::Fri => "星期五",
            chrono::Weekday::Sat => "星期六",
            chrono::Weekday::Sun => "星期日",
        }
        .to_string()
    }

    /// 获取当前是星期几（英文缩写）
    ///
    /// # Returns
    ///
    /// "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"。
    ///
    /// # Get Day of Week in English (Abbreviated)
    ///
    /// # Returns
    ///
    /// Abbreviated English day name: "Mon", "Tue", etc.
    pub fn day_of_week_en() -> String {
        Self::_now().format("%a").to_string()
    }

    /// 获取当前年份
    ///
    /// # Returns
    ///
    /// 当前年份（例如：2025）。
    ///
    /// # Get Current Year
    ///
    /// # Returns
    ///
    /// The current year (e.g., 2025).
    pub fn year() -> i32 {
        Self::_now().year()
    }

    /// 获取当前月份
    ///
    /// # Returns
    ///
    /// 1 到 12 之间的月份数字。
    ///
    /// # Get Current Month
    ///
    /// # Returns
    ///
    /// Month number between 1 and 12.
    pub fn month() -> u32 {
        Self::_now().month()
    }

    /// 获取当前是本月的第几天
    ///
    /// # Returns
    ///
    /// 1 到 31 之间的日期数字。
    ///
    /// # Get Day of Month
    ///
    /// # Returns
    ///
    /// Day of the month (1-31).
    pub fn day() -> u32 {
        Self::_now().day()
    }

    /// 获取当前小时（24小时制）
    ///
    /// # Returns
    ///
    /// 0 到 23 之间的小时数。
    ///
    /// # Get Hour (24-hour clock)
    ///
    /// # Returns
    ///
    /// Hour of the day (0-23).
    pub fn hour() -> u32 {
        Self::_now().hour()
    }

    /// 获取当前分钟
    ///
    /// # Returns
    ///
    /// 0 到 59 之间的分钟数。
    ///
    /// # Get Minute
    ///
    /// # Returns
    ///
    /// Minute of the hour (0-59).
    pub fn minute() -> u32 {
        Self::_now().minute()
    }

    /// 获取当前秒数
    ///
    /// # Returns
    ///
    /// 0 到 59 之间的秒数（不包括闰秒）。
    ///
    /// # Get Second
    ///
    /// # Returns
    ///
    /// Second of the minute (0-59, excluding leap seconds).
    pub fn second() -> u32 {
        Self::_now().second()
    }

    /// 将日期字符串按指定格式解析为 `NaiveDate`
    ///
    /// # Parameters
    ///
    /// * `date_str` - 要解析的日期字符串。
    /// * `format` - 用于解析的格式字符串。
    ///
    /// # Returns
    ///
    /// 解析成功返回 `Some(NaiveDate)`，失败返回 `None`。
    ///
    /// # Parse Date String with Format
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

    /// 计算指定天数前的日期
    ///
    /// # Parameters
    ///
    /// * `days` - 要减去的天数。
    ///
    /// # Returns
    ///
    /// 指定天数前的日期字符串（格式：YYYY-MM-DD）。
    ///
    /// # Get Date Before N Days
    ///
    /// # Parameters
    ///
    /// * `days` - Number of days to subtract.
    ///
    /// # Returns
    ///
    /// Date string (format: YYYY-MM-DD) from N days ago.
    pub fn date_before(days: u64) -> String {
        let now = Self::_now();
        let date = now.date_naive() - chrono::Duration::days(days as i64);
        date.format("%Y-%m-%d").to_string()
    }

    /// 计算指定天数后的日期
    ///
    /// # Parameters
    ///
    /// * `days` - 要增加的天数。
    ///
    /// # Returns
    ///
    /// 指定天数后的日期字符串（格式：YYYY-MM-DD）。
    ///
    /// # Get Date After N Days
    ///
    /// # Parameters
    ///
    /// * `days` - Number of days to add.
    ///
    /// # Returns
    ///
    /// Date string (format: YYYY-MM-DD) from N days in the future.
    pub fn date_after(days: u64) -> String {
        let now = Self::_now();
        let date = now.date_naive() + chrono::Duration::days(days as i64);
        date.format("%Y-%m-%d").to_string()
    }
}