use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use std::fmt;

/// Cron 表达式结构
/// Cron expression structure
#[derive(Debug, Clone)]
pub struct CronExpression {
    pub minute: Vec<u8>,      // 分钟 (0-59) / Minutes (0-59)
    pub hour: Vec<u8>,        // 小时 (0-23) / Hours (0-23)
    pub day_of_month: Vec<u8>, // 日期 (1-31) / Day of month (1-31)
    pub month: Vec<u8>,       // 月份 (1-12) / Month (1-12)
    pub day_of_week: Vec<u8>, // 星期 (0-6 或 1-7，0/7 都表示星期日) / Day of week (0-6 or 1-7, 0/7 both represent Sunday)
    pub expression: String,   // 原始表达式 / Original expression
}

/// Cron 表达式解析错误
/// Cron expression parsing error
#[derive(Debug)]
pub enum CronError {
    InvalidExpression(String),
    InvalidField(String),
    InvalidRange(String),
    InvalidValue(String),
}

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CronError::InvalidExpression(msg) => write!(f, "Invalid cron expression: {}", msg),
            CronError::InvalidField(msg) => write!(f, "Invalid cron field: {}", msg),
            CronError::InvalidRange(msg) => write!(f, "Invalid range: {}", msg),
            CronError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
        }
    }
}

pub struct CronUtil;

impl CronUtil {
    /// 解析 cron 表达式为结构体
    /// Parse cron expression into a struct
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式，如 "0 0 * * *"
    /// * `expression` - cron expression, e.g. "0 0 * * *"
    /// 
    /// # 示例
    /// # Example
    /// ```
    /// let cron = CronUtil::parse("0 0 * * *").unwrap();
    /// ```
    pub fn parse(expression: &str) -> Result<CronExpression, CronError> {
        let parts: Vec<&str> = expression.split_whitespace().collect();
        
        if parts.len() != 5 {
            return Err(CronError::InvalidExpression(
                format!("Expected to have 5 fields, actually there are {}", parts.len())
            ));
        }
        
        let minute = Self::parse_field(parts[0], 0, 59)?;
        let hour = Self::parse_field(parts[1], 0, 23)?;
        let day_of_month = Self::parse_field(parts[2], 1, 31)?;
        let month = Self::parse_field(parts[3], 1, 12)?;
        let day_of_week = Self::parse_field(parts[4], 0, 6)?;
        
        Ok(CronExpression {
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
            expression: expression.to_string(),
        })
    }
    
    /// 解析 cron 字段
    /// Parse cron field
    fn parse_field(field: &str, min: u8, max: u8) -> Result<Vec<u8>, CronError> {
        if field == "*" {
            return Ok((min..=max).collect());
        }
        
        let mut values = Vec::new();
        
        for part in field.split(',') {
            if part.contains('/') {
                // 处理步长: */5, 1-30/5
                // Handle step: */5, 1-30/5
                let segments: Vec<&str> = part.split('/').collect();
                if segments.len() != 2 {
                    return Err(CronError::InvalidField(format!("Invalid step expression: {}", part)));
                }
                
                let range = segments[0];
                let step = segments[1].parse::<u8>().map_err(|_| {
                    CronError::InvalidValue(format!("Invalid step value: {}", segments[1]))
                })?;
                
                if step == 0 {
                    return Err(CronError::InvalidValue("Step cannot be 0".to_string()));
                }
                
                let range_values = if range == "*" {
                    (min..=max).collect::<Vec<u8>>()
                } else if range.contains('-') {
                    Self::parse_range(range, min, max)?
                } else {
                    let value = range.parse::<u8>().map_err(|_| {
                        CronError::InvalidValue(format!("Invalid value: {}", range))
                    })?;
                    Self::validate_range(value, min, max)?;
                    vec![value]
                };
                
                for i in (0..range_values.len()).step_by(step as usize) {
                    values.push(range_values[i]);
                }
            } else if part.contains('-') {
                // 处理范围: 1-5
                // Handle range: 1-5
                let range_values = Self::parse_range(part, min, max)?;
                values.extend(range_values);
            } else {
                // 单一值
                // Single value
                let value = part.parse::<u8>().map_err(|_| {
                    CronError::InvalidValue(format!("Invalid field value: {}", part))
                })?;
                Self::validate_range(value, min, max)?;
                values.push(value);
            }
        }
        
        // 去重并排序
        // Remove duplicates and sort
        values.sort();
        values.dedup();
        
        Ok(values)
    }
    
    /// 解析范围表达式
    /// Parse range expression
    fn parse_range(range: &str, min: u8, max: u8) -> Result<Vec<u8>, CronError> {
        let parts: Vec<&str> = range.split('-').collect();
        if parts.len() != 2 {
            return Err(CronError::InvalidRange(format!("Invalid range expression: {}", range)));
        }
        
        let start = parts[0].parse::<u8>().map_err(|_| {
            CronError::InvalidValue(format!("Invalid range start value: {}", parts[0]))
        })?;
        
        let end = parts[1].parse::<u8>().map_err(|_| {
            CronError::InvalidValue(format!("Invalid range end value: {}", parts[1]))
        })?;
        
        Self::validate_range(start, min, max)?;
        Self::validate_range(end, min, max)?;
        
        if start > end {
            return Err(CronError::InvalidRange(
                format!("Range start value ({}) greater than end value ({})", start, end)
            ));
        }
        
        Ok((start..=end).collect())
    }
    
    /// 验证值是否在允许范围内
    /// Validate if value is within allowed range
    fn validate_range(value: u8, min: u8, max: u8) -> Result<(), CronError> {
        if value < min || value > max {
            return Err(CronError::InvalidRange(
                format!("Value {} out of allowed range {}-{}", value, min, max)
            ));
        }
        Ok(())
    }
    
    /// 验证 cron 表达式是否有效
    /// Validate if a cron expression is valid
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式，如 "0 0 * * *"
    /// * `expression` - cron expression, e.g. "0 0 * * *"
    /// 
    /// # 示例
    /// # Example
    /// ```
    /// if CronUtil::validate("0 0 * * *") {
    ///     println!("Valid cron expression");
    /// }
    /// ```
    pub fn validate(expression: &str) -> bool {
        Self::parse(expression).is_ok()
    }
    
    /// 从当前时间计算下一次执行时间
    /// Calculate next execution time from current time
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式
    /// * `expression` - cron expression
    /// 
    /// # 示例
    /// # Example
    /// ```
    /// let next = CronUtil::next_execution("0 0 * * *").unwrap();
    /// println!("Next execution time: {}", next);
    /// ```
    pub fn next_execution(expression: &str) -> Result<DateTime<Local>, CronError> {
        let now = Local::now();
        Self::next_execution_from(expression, now)
    }
    
    /// 从指定时间计算下一次执行时间
    /// Calculate next execution time from a specific time
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式
    /// * `expression` - cron expression
    /// * `from` - 起始时间
    /// * `from` - start time
    pub fn next_execution_from(expression: &str, from: DateTime<Local>) -> Result<DateTime<Local>, CronError> {
        let cron = Self::parse(expression)?;
        
        let mut candidate = from + Duration::minutes(1);
        candidate = candidate
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        
        // 最多迭代 2 年，避免无限循环
        // Iterate at most 2 years to avoid infinite loop
        let limit = from + Duration::days(366 * 2);
        
        while candidate < limit {
            let month = candidate.month() as u8;
            if !cron.month.contains(&month) {
                candidate = Self::add_months(candidate, 1).with_day(1).unwrap();
                continue;
            }
            
            let dom = candidate.day() as u8;
            if !cron.day_of_month.contains(&dom) {
                candidate = candidate + Duration::days(1);
                candidate = candidate
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap();
                continue;
            }
            
            let dow = candidate.weekday().num_days_from_sunday() as u8;
            if !cron.day_of_week.contains(&dow) && !cron.day_of_week.contains(&7) {
                candidate = candidate + Duration::days(1);
                candidate = candidate
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap();
                continue;
            }
            
            let hour = candidate.hour() as u8;
            if !cron.hour.contains(&hour) {
                candidate = candidate + Duration::hours(1);
                candidate = candidate.with_minute(0).unwrap();
                continue;
            }
            
            let minute = candidate.minute() as u8;
            if !cron.minute.contains(&minute) {
                candidate = candidate + Duration::minutes(1);
                continue;
            }
            
            return Ok(candidate);
        }
        
        Err(CronError::InvalidExpression("Cannot find next execution time, expression may be invalid or never executes".to_string()))
    }
    
    /// 添加月数到日期
    /// Add months to a date
    fn add_months(dt: DateTime<Local>, months: i32) -> DateTime<Local> {
        let mut year = dt.year();
        let mut month = dt.month() as i32 + months;
        
        while month > 12 {
            year += 1;
            month -= 12;
        }
        
        while month < 1 {
            year -= 1;
            month += 12;
        }
        
        dt.with_year(year)
          .unwrap()
          .with_month(month as u32)
          .unwrap()
    }
    
    /// 生成 cron 表达式的人类可读描述
    /// Generate human-readable description of a cron expression
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式
    /// * `expression` - cron expression
    /// 
    /// # 示例
    /// # Example
    /// ```
    /// let desc = CronUtil::describe("0 0 * * *").unwrap();
    /// println!("{}", desc); // "Every day at 00:00"
    /// ```
    pub fn describe(expression: &str) -> Result<String, CronError> {
        let cron = Self::parse(expression)?;
        
        // 特殊情况处理
        // Handle special cases
        if expression == "* * * * *" {
            return Ok("Every minute".to_string());
        }
        
        if expression == "0 * * * *" {
            return Ok("Every hour at the top of the hour".to_string());
        }
        
        if expression == "0 0 * * *" {
            return Ok("Every day at 00:00".to_string());
        }
        
        // 一般描述生成
        // General description generation
        let mut desc = String::new();
        
        // 处理星期
        // Handle day of week
        if cron.day_of_week.len() < 7 {
            if cron.day_of_week.len() == 1 {
                desc.push_str(&format!("Every {} ", Self::day_of_week_name_en(cron.day_of_week[0])));
            } else {
                desc.push_str("Every week on ");
                for &day in &cron.day_of_week {
                    desc.push_str(&format!("{}, ", Self::day_of_week_name_en(day)));
                }
                desc.pop(); // 移除最后的逗号 / Remove last comma
                desc.pop(); // 移除最后的空格 / Remove last space
                desc.push(' ');
            }
        } else {
            desc.push_str("Every day ");
        }
        
        // 处理月份
        // Handle month
        if cron.month.len() < 12 {
            if cron.month.len() == 1 {
                desc.push_str(&format!("in {} ", Self::month_name_en(cron.month[0])));
            } else {
                desc.push_str("in ");
                for &month in &cron.month {
                    desc.push_str(&format!("{}, ", Self::month_name_en(month)));
                }
                desc.pop(); // 移除最后的逗号 / Remove last comma
                desc.pop(); // 移除最后的空格 / Remove last space
                desc.push(' ');
            }
        }
        
        // 处理时间
        // Handle time
        if cron.hour.len() == 1 && cron.minute.len() == 1 {
            desc.push_str(&format!("at {:02}:{:02}", cron.hour[0], cron.minute[0]));
        } else if cron.hour.len() == 1 {
            desc.push_str(&format!("at {:02}:xx", cron.hour[0]));
            if cron.minute.len() <= 5 {
                desc.push_str(" (minutes: ");
                for &min in &cron.minute {
                    desc.push_str(&format!("{:02}, ", min));
                }
                desc.pop(); // 移除最后的逗号 / Remove last comma
                desc.pop(); // 移除最后的空格 / Remove last space
                desc.push(')');
            }
        } else {
            if cron.hour.len() <= 5 {
                desc.push_str("at hours: ");
                for &hour in &cron.hour {
                    desc.push_str(&format!("{:02}, ", hour));
                }
                desc.pop(); // 移除最后的逗号 / Remove last comma
                desc.pop(); // 移除最后的空格 / Remove last space
            } else {
                desc.push_str("every hour");
            }
            
            if cron.minute.len() == 1 {
                desc.push_str(&format!(", minute: {:02}", cron.minute[0]));
            } else if cron.minute.len() <= 5 {
                desc.push_str(", minutes: ");
                for &min in &cron.minute {
                    desc.push_str(&format!("{:02}, ", min));
                }
                desc.pop(); // 移除最后的逗号 / Remove last comma
                desc.pop(); // 移除最后的空格 / Remove last space
            }
        }
        
        Ok(desc)
    }
    
    /// 获取星期几的中文名称
    /// Get Chinese name for day of week
    fn day_of_week_name(day: u8) -> &'static str {
        match day {
            0 | 7 => "星期日",
            1 => "星期一",
            2 => "星期二",
            3 => "星期三",
            4 => "星期四",
            5 => "星期五",
            6 => "星期六",
            _ => "未知",
        }
    }
    
    /// 获取星期几的英文名称
    /// Get English name for day of week
    fn day_of_week_name_en(day: u8) -> &'static str {
        match day {
            0 | 7 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Unknown",
        }
    }
    
    /// 获取月份的英文名称
    /// Get English name for month
    fn month_name_en(month: u8) -> &'static str {
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
            _ => "Unknown",
        }
    }
    
    /// 计算两个日期之间有多少次 cron 触发
    /// Count number of cron executions between two dates
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式
    /// * `expression` - cron expression
    /// * `start` - 开始时间
    /// * `start` - start time
    /// * `end` - 结束时间
    /// * `end` - end time
    pub fn count_executions_between(
        expression: &str, 
        start: DateTime<Local>, 
        end: DateTime<Local>
    ) -> Result<u32, CronError> {
        if start >= end {
            return Ok(0);
        }
        
        let mut count = 0;
        let mut current = start;
        
        while current < end {
            match Self::next_execution_from(expression, current) {
                Ok(next) => {
                    if next >= end {
                        break;
                    }
                    count += 1;
                    current = next + Duration::minutes(1);
                },
                Err(_) => break,
            }
        }
        
        Ok(count)
    }
    
    /// 获取未来 n 次执行时间
    /// Get next n execution times
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式
    /// * `expression` - cron expression
    /// * `count` - 需要获取的次数
    /// * `count` - number of times to get
    pub fn next_n_executions(
        expression: &str, 
        count: usize
    ) -> Result<Vec<DateTime<Local>>, CronError> {
        let mut executions = Vec::with_capacity(count);
        let mut current = Local::now();
        
        for _ in 0..count {
            match Self::next_execution_from(expression, current) {
                Ok(next) => {
                    executions.push(next);
                    current = next + Duration::minutes(1);
                },
                Err(e) => return Err(e),
            }
        }
        
        Ok(executions)
    }
    
    /// 根据 cron 表达式判断某个时间点是否会触发
    /// Check if a specific time matches a cron expression
    /// 
    /// # 参数
    /// # Parameters
    /// * `expression` - cron 表达式
    /// * `expression` - cron expression
    /// * `date_time` - 待检查的时间点
    /// * `date_time` - time to check
    pub fn matches(expression: &str, date_time: DateTime<Local>) -> Result<bool, CronError> {
        let cron = Self::parse(expression)?;
        
        let minute = date_time.minute() as u8;
        let hour = date_time.hour() as u8;
        let day = date_time.day() as u8;
        let month = date_time.month() as u8;
        let dow = date_time.weekday().num_days_from_sunday() as u8;
        
        // DOM 和 DOW 是 OR 关系
        // DOM and DOW have an OR relationship
        let day_matches = cron.day_of_month.contains(&day) || 
                         cron.day_of_week.contains(&dow) || 
                         (dow == 0 && cron.day_of_week.contains(&7));
        
        Ok(cron.minute.contains(&minute) && 
           cron.hour.contains(&hour) && 
           cron.month.contains(&month) && 
           day_matches)
    }
    
    /// 解析常用的 cron 表达式别名
    /// Parse common cron expression aliases
    /// 
    /// # 参数
    /// # Parameters 
    /// * `alias` - cron 表达式别名，如 "@daily", "@hourly"
    /// * `alias` - cron expression alias, e.g. "@daily", "@hourly"
    pub fn parse_alias(alias: &str) -> Result<String, CronError> {
        match alias {
            "@yearly" | "@annually" => Ok("0 0 1 1 *".to_string()),
            "@monthly" => Ok("0 0 1 * *".to_string()),
            "@weekly" => Ok("0 0 * * 0".to_string()),
            "@daily" | "@midnight" => Ok("0 0 * * *".to_string()),
            "@hourly" => Ok("0 * * * *".to_string()),
            _ => Err(CronError::InvalidExpression(format!("Unknown cron alias: {}", alias))),
        }
    }
    
    /// 根据给定的月、周、日、时、分生成 cron 表达式
    /// Generate cron expression from given month, week day, day, hour and minute
    /// 
    /// # 参数
    /// # Parameters
    /// * `month` - 月份，1-12，0 表示所有月份
    /// * `month` - Month, 1-12, 0 means all months
    /// * `week_day` - 星期几，0-7（0 和 7 都表示星期日），-1 表示所有星期
    /// * `week_day` - Day of week, 0-7 (0 and 7 both represent Sunday), -1 means all days
    /// * `day` - 日期，1-31，0 表示所有日期
    /// * `day` - Day of month, 1-31, 0 means all days
    /// * `hour` - 小时，0-23，-1 表示所有小时
    /// * `hour` - Hour, 0-23, -1 means all hours
    /// * `minute` - 分钟，0-59，-1 表示所有分钟
    /// * `minute` - Minute, 0-59, -1 means all minutes
    /// 
    /// # 返回
    /// # Returns
    /// * `String` - 生成的 cron 表达式
    /// * `String` - Generated cron expression
    pub fn generate_cron(month: i32, week_day: i32, day: i32, hour: i32, minute: i32) -> String {
        let minute_str = if minute < 0 || minute > 59 { "*".to_string() } else { minute.to_string() };
        let hour_str = if hour < 0 || hour > 23 { "*".to_string() } else { hour.to_string() };
        let day_str = if day < 1 || day > 31 { "*".to_string() } else { day.to_string() };
        
        let month_str = if month < 1 || month > 12 { 
            "*".to_string() 
        } else { 
            month.to_string() 
        };
        
        let week_day_str = if week_day < 0 || week_day > 7 { 
            "*".to_string() 
        } else { 
            week_day.to_string() 
        };
        
        format!("{} {} {} {} {}", minute_str, hour_str, day_str, month_str, week_day_str)
    }
}



