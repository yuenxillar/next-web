use chrono::{DateTime, Local};

pub struct LocalDateTime;

impl LocalDateTime {
    pub fn now() -> String {
        let now: DateTime<Local> = Local::now();
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
