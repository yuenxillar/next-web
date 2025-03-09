use chrono::{DateTime, Local};

/**
*struct:    LocalDateTimeUtil
*desc:      本地日期时间工具类
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct LocalDateTimeUtil;

impl LocalDateTimeUtil {
    pub fn now() -> String {
        let now: DateTime<Local> = Local::now();
        // 时间格式化
        return now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    }

    pub fn timestamp() -> i64 {
        let now: DateTime<Local> = Local::now();
        return now.timestamp();
    }

    pub fn date() -> String {
        let now: DateTime<Local> = Local::now();
        return now.format("%Y-%m-%d").to_string();
    }

    pub fn to_save_path() -> String {
        let now: DateTime<Local> = Local::now();
        return now.format("%Y/%m/%d").to_string();
    }
}
