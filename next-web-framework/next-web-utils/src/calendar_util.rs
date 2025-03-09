use chrono::NaiveDate;

/**
*struct:    CalenderUtil
*desc:      日历工具类
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct CalenderUtil;

impl CalenderUtil {
    pub fn get_week_days<'a>() -> Vec<&'a str> {
        vec!["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    }

    pub fn calculate_days(year: i32, month: u32, day: u32, offset: i64) -> Result<NaiveDate, ()> {
        if let Some(start_date) = NaiveDate::from_ymd_opt(year, month, day) {
            return Ok(start_date - chrono::Duration::days(offset));
        };
        Err(())
    }

    pub fn calculate_weeks(year: i32, month: u32, day: u32, offset: i64) -> Result<NaiveDate, ()> {
        if let Some(start_date) = NaiveDate::from_ymd_opt(year, month, day) {
            return Ok(start_date - chrono::Duration::weeks(offset));
        };
        Err(())
    }
}
