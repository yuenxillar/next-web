use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Weekday};
use std::collections::{HashMap, HashSet};

/// 日历视图类型
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CalendarViewType {
    Day,
    Week,
    Month,
    Year,
}

/// 日历事件
#[derive(Clone, Debug)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub all_day: bool,
    pub description: Option<String>,
    pub location: Option<String>,
    pub color: Option<String>,    // 事件颜色
    pub repeated: bool,           // 是否重复事件
    pub repeat_rule: Option<String>, // 重复规则
    pub reminder: Option<i64>,    // 提前提醒的分钟数
    pub tags: Vec<String>,        // 事件标签
}

/// 月份日历数据
#[derive(Debug)]
pub struct MonthCalendar {
    pub year: i32,
    pub month: u32,
    pub weeks: Vec<Vec<DayInfo>>, // 按周存储的天信息
    pub total_days: u32,
}

/// 单日信息
#[derive(Debug, Clone)]
pub struct DayInfo {
    pub date: NaiveDate,
    pub is_today: bool,
    pub is_current_month: bool,
    pub is_weekend: bool,
    pub is_holiday: bool,
    pub holiday_name: Option<String>,
    pub lunar_date: Option<String>,     // 农历日期
    pub solar_term: Option<String>,     // 节气
    pub events: Vec<CalendarEvent>,
}

/// 周信息
#[derive(Debug)]
pub struct WeekInfo {
    pub year: i32,
    pub week_number: u32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub days: Vec<DayInfo>,
}

/// 节假日信息
#[derive(Debug, Clone)]
pub struct HolidayInfo {
    pub name: String,
    pub date: NaiveDate,
    pub is_work_day: bool,
}

/// Web 日历工具类 - 简化版
pub struct CalendarUtil;

impl CalendarUtil {
    
    /// 获取月份名称（中文）
    pub fn month_name(month: u32) -> &'static str {
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
    
    /// 获取月份名称（英文）
    pub fn month_name_en(month: u32) -> &'static str {
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
    
    /// 获取星期名称（中文）
    pub fn weekday_name(weekday: &Weekday) -> &'static str {
        match weekday {
            Weekday::Mon => "星期一",
            Weekday::Tue => "星期二",
            Weekday::Wed => "星期三",
            Weekday::Thu => "星期四",
            Weekday::Fri => "星期五",
            Weekday::Sat => "星期六",
            Weekday::Sun => "星期日",
        }
    }
    
    /// 获取星期名称（英文）
    pub fn weekday_name_en(weekday: &Weekday) -> &'static str {
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
    
    /// 获取星期简称（中文）
    pub fn weekday_short_name(weekday: &Weekday) -> &'static str {
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
    
    /// 获取星期简称（英文）
    pub fn weekday_short_name_en(weekday: &Weekday) -> &'static str {
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

    
    /// 获取某月中的天数
    pub fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => panic!("无效月份"),
        }
    }
    
    /// 检查是否为闰年
    pub fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
    
    /// 获取一年中的季度数
    pub fn quarters_in_year() -> u32 {
        4
    }
    
    /// 获取季度包含的月份
    pub fn months_in_quarter(quarter: u32) -> Vec<u32> {
        match quarter {
            1 => vec![1, 2, 3],
            2 => vec![4, 5, 6],
            3 => vec![7, 8, 9],
            4 => vec![10, 11, 12],
            _ => panic!("无效季度"),
        }
    }
    
    /// 获取某日在一年中是第几天
    pub fn day_of_year(date: NaiveDate) -> u32 {
        date.ordinal()
    }
    
    /// 获取某日在一周中是第几天（周一为1，周日为7）
    pub fn day_of_week(date: NaiveDate) -> u32 {
        date.weekday().number_from_monday()
    }
    
    /// 获取某日在一年中是第几周
    pub fn week_of_year(date: NaiveDate) -> u32 {
        date.iso_week().week()
    }
    
    /// 获取某月的第一天
    pub fn first_day_of_month(year: i32, month: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, 1).unwrap()
    }
    
    /// 获取某月的最后一天
    pub fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
        let days = Self::days_in_month(year, month);
        NaiveDate::from_ymd_opt(year, month, days).unwrap()
    }
    
    /// 获取某季度的第一天
    pub fn first_day_of_quarter(year: i32, quarter: u32) -> NaiveDate {
        let month = (quarter - 1) * 3 + 1;
        NaiveDate::from_ymd_opt(year, month, 1).unwrap()
    }
    
    /// 获取某季度的最后一天
    pub fn last_day_of_quarter(year: i32, quarter: u32) -> NaiveDate {
        let month = quarter * 3;
        Self::last_day_of_month(year, month)
    }
    
    /// 获取某年的第一天
    pub fn first_day_of_year(year: i32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
    }
    
    /// 获取某年的最后一天
    pub fn last_day_of_year(year: i32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, 12, 31).unwrap()
    }
    
    /// 判断是否为周末
    pub fn is_weekend(date: NaiveDate) -> bool {
        date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun
    }
    
    /// 判断是否为工作日（简单判断，仅排除周末）
    pub fn is_weekday(date: NaiveDate) -> bool {
        !Self::is_weekend(date)
    }
    
    /// 获取当月日历网格（用于渲染）
    pub fn get_month_grid(year: i32, month: u32) -> Vec<Vec<Option<u32>>> {
        let first_day = Self::first_day_of_month(year, month);
        let days_in_month = Self::days_in_month(year, month);
        
        // 当月第一天是星期几（0-6，对应周一到周日）
        let first_weekday = first_day.weekday().num_days_from_monday();
        
        // 构建网格（最多6行，每行7天）
        let mut grid: Vec<Vec<Option<u32>>> = Vec::new();
        let mut week: Vec<Option<u32>> = Vec::new();
        
        // 填充第一周前面的空白
        for _ in 0..first_weekday {
            week.push(None);
        }
        
        // 填充当月日期
        for day in 1..=days_in_month {
            week.push(Some(day));
            
            // 一周结束或月末
            if week.len() == 7 {
                grid.push(week);
                week = Vec::new();
            }
        }
        
        // 填充最后一周剩余的空白
        while !week.is_empty() && week.len() < 7 {
            week.push(None);
        }
        
        if !week.is_empty() {
            grid.push(week);
        }
        
        grid
    }
    
    /// 获取两个日期之间的天数
    pub fn days_between(start: NaiveDate, end: NaiveDate) -> i64 {
        (end - start).num_days()
    }
    
    /// 获取两个日期之间的工作日数（简单计算，仅排除周末）
    pub fn weekdays_between(start: NaiveDate, end: NaiveDate) -> i64 {
        let mut count = 0;
        let mut current = start;
        
        while current <= end {
            if Self::is_weekday(current) {
                count += 1;
            }
            current = current.succ_opt().unwrap();
        }
        
        count
    }
    
    /// 检查两个日期是否在同一个月
    pub fn is_same_month(date1: NaiveDate, date2: NaiveDate) -> bool {
        date1.year() == date2.year() && date1.month() == date2.month()
    }
    
    /// 检查两个日期是否在同一年
    pub fn is_same_year(date1: NaiveDate, date2: NaiveDate) -> bool {
        date1.year() == date2.year()
    }
    
    /// 将日期格式化为特定格式
    pub fn format_date(date: NaiveDate, format: &str) -> String {
        date.format(format).to_string()
    }
    
    /// 获取今天的日期
    pub fn today() -> NaiveDate {
        Local::now().date_naive()
    }
    
    /// 获取本周的第一天（周一）
    pub fn first_day_of_current_week() -> NaiveDate {
        let today = Self::today();
        let weekday = today.weekday().num_days_from_monday();
        today - Duration::days(weekday as i64)
    }
    
    /// 获取上个月的同一天
    pub fn same_day_last_month(date: NaiveDate) -> Option<NaiveDate> {
        let year = if date.month() == 1 {
            date.year() - 1
        } else {
            date.year()
        };
        
        let month = if date.month() == 1 {
            12
        } else {
            date.month() - 1
        };
        
        let last_day = Self::days_in_month(year, month);
        let day = std::cmp::min(date.day(), last_day);
        
        NaiveDate::from_ymd_opt(year, month, day)
    }
    
    /// 获取下个月的同一天
    pub fn same_day_next_month(date: NaiveDate) -> Option<NaiveDate> {
        let year = if date.month() == 12 {
            date.year() + 1
        } else {
            date.year()
        };
        
        let month = if date.month() == 12 {
            1
        } else {
            date.month() + 1
        };
        
        let last_day = Self::days_in_month(year, month);
        let day = std::cmp::min(date.day(), last_day);
        
        NaiveDate::from_ymd_opt(year, month, day)
    }
}

/// 农历日历实现
pub struct LunarCalendar {
    /// 农历数据表 (1900-2100)
    /// 每个数字的二进制表示：
    /// 1-4位：表示闰月的月份，为0表示没有闰月
    /// 5-16位：表示12个月的大小月情况，1为大月(30天)，0为小月(29天)
    /// 17-20位：表示闰月的大小月情况，1为大月，0为小月
    lunar_info: Vec<u32>,
    
    /// 天干
    heavenly_stems: Vec<&'static str>,
    
    /// 地支
    earthly_branches: Vec<&'static str>,
    
    /// 生肖
    zodiac_animals: Vec<&'static str>,
    
    /// 农历月份
    lunar_months: Vec<&'static str>,
    
    /// 农历日
    lunar_days: Vec<&'static str>,
    
    /// 24节气
    solar_terms: Vec<&'static str>,
    
    /// 节气日期表(以春分点为例：每年春分时刻按角度计算)
    solar_term_base: Vec<f64>,
}

impl LunarCalendar {
    pub fn new() -> Self {
        LunarCalendar {
            // 农历数据表 (1900-2100)
            // 数据略长，这里仅展示部分
            lunar_info: vec![
                0x04bd8, 0x04ae0, 0x0a570, 0x054d5, 0x0d260, 0x0d950, 0x16554, 0x056a0, 0x09ad0, 0x055d2,
                0x04ae0, 0x0a5b6, 0x0a4d0, 0x0d250, 0x1d255, 0x0b540, 0x0d6a0, 0x0ada2, 0x095b0, 0x14977,
                0x04970, 0x0a4b0, 0x0b4b5, 0x06a50, 0x06d40, 0x1ab54, 0x02b60, 0x09570, 0x052f2, 0x04970,
                0x06566, 0x0d4a0, 0x0ea50, 0x06e95, 0x05ad0, 0x02b60, 0x186e3, 0x092e0, 0x1c8d7, 0x0c950,
                0x0d4a0, 0x1d8a6, 0x0b550, 0x056a0, 0x1a5b4, 0x025d0, 0x092d0, 0x0d2b2, 0x0a950, 0x0b557,
                // 更多数据...
            ],
            heavenly_stems: vec!["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"],
            earthly_branches: vec!["子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥"],
            zodiac_animals: vec!["鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪"],
            lunar_months: vec!["正月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "冬月", "腊月"],
            lunar_days: vec![
                "初一", "初二", "初三", "初四", "初五", "初六", "初七", "初八", "初九", "初十",
                "十一", "十二", "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十",
                "廿一", "廿二", "廿三", "廿四", "廿五", "廿六", "廿七", "廿八", "廿九", "三十"
            ],
            solar_terms: vec![
                "小寒", "大寒", "立春", "雨水", "惊蛰", "春分", 
                "清明", "谷雨", "立夏", "小满", "芒种", "夏至", 
                "小暑", "大暑", "立秋", "处暑", "白露", "秋分", 
                "寒露", "霜降", "立冬", "小雪", "大雪", "冬至"
            ],
            // 节气基准日期 (2000年的节气点，单位：分钟)
            solar_term_base: vec![
                6.11, 20.84, 4.6295, 19.4599, 6.3826, 21.4155,
                5.59, 20.888, 6.318, 21.86, 6.5, 22.20, 
                7.928, 23.65, 8.35, 23.95, 8.44, 23.822, 
                9.098, 24.218, 8.218, 23.08, 7.9, 22.6
            ],
        }
    }
    
    /// 获取特定日期的农历表示
    pub fn get_lunar_date(&self, date: NaiveDate) -> String {
        // 计算与农历基准日期的差值 (1900-01-31是农历1900年正月初一)
        let base_date = NaiveDate::from_ymd_opt(1900, 1, 31).unwrap();
        let mut offset = (date - base_date).num_days() as i64;
        
        if offset < 0 {
            return "不支持1900年以前的日期".to_string();
        }
        
        // 计算农历年
        let mut i = 0;
        let mut leap_day = 0;
        let mut leap_month = 0;
        let mut temp = 0;
        let mut year_days = 0;
        
        while i < 200 { // 最多搜索200年
            // 计算农历一年的总天数
            temp = self.lunar_year_days(1900 + i);
            leap_day = temp & 0xf; // 获取闰月天数
            leap_month = (temp >> 16) & 0xf; // 获取闰月月份
            year_days = if leap_month > 0 { 
                (temp >> 4) & 0xfff 
            } else { 
                (temp >> 4) & 0xfff + leap_day 
            };
            
            if offset < year_days as i64 {
                break;
            }
            
            offset -= year_days as i64;
            i += 1;
        }
        
        let lunar_year = 1900 + i;
        
        // 计算月份和日期
        let mut lunar_month = 1;
        let mut is_leap = false;
        
        // 计算月份
        let lunar_info = self.lunar_info[i as usize];
        
        for j in 0..13 {
            // 闰月
            if leap_month > 0 && j == leap_month {
                is_leap = true;
                let days = if (lunar_info >> (16 + leap_month)) & 0x1 == 1 { 30 } else { 29 };
                if offset < days as i64 {
                    break;
                }
                offset -= days as i64;
            }
            
            // 计算正常月天数
            let month_days = if (lunar_info >> (16 - j)) & 0x1 == 1 { 30 } else { 29 };
            if offset < month_days as i64 {
                break;
            }
            
            offset -= month_days as i64;
            
            // 如果不是闰月，递增月份
            if !is_leap {
                lunar_month += 1;
            } else {
                is_leap = false;
            }
        }
        
        let lunar_day = offset as u32 + 1;
        
        // 格式化输出
        let month_str = if is_leap {
            format!("闰{}", self.lunar_months[(lunar_month - 1) as usize])
        } else {
            self.lunar_months[(lunar_month - 1) as usize].to_string()
        };
        
        let day_str = self.lunar_days[(lunar_day - 1) as usize];
        
        // 农历年干支
        let cycle_year = (lunar_year - 1864) % 60;
        let gan_index = (cycle_year % 10) as usize;
        let zhi_index = (cycle_year % 12) as usize;
        
        format!("{}{}年{}{}",
            self.heavenly_stems[gan_index],
            self.earthly_branches[zhi_index],
            month_str,
            day_str
        )
    }
    
    /// 计算农历年的总天数
    fn lunar_year_days(&self, year: i32) -> u32 {
        let idx = (year - 1900) as usize;
        if idx >= self.lunar_info.len() {
            return 0;
        }
        
        let lunar_info = self.lunar_info[idx];
        
        // 计算12个月的天数总和
        let mut sum = 0;
        for i in 0..12 {
            sum += if (lunar_info >> (16 - i)) & 0x1 == 1 { 30 } else { 29 };
        }
        
        // 加上闰月的天数
        let leap_month = (lunar_info >> 20) & 0xf;
        if leap_month > 0 {
            sum += if (lunar_info >> (16 + leap_month)) & 0x1 == 1 { 30 } else { 29 };
        }
        
        // 返回总天数和闰月信息
        (sum << 4) | leap_month << 16 | (if leap_month > 0 { (lunar_info >> (16 + leap_month)) & 0x1 } else { 0 })
    }
    
    /// 获取特定日期的节气信息
    pub fn get_solar_term(&self, date: NaiveDate) -> Option<String> {
        let year = date.year();
        let month = date.month() as usize;
        let day = date.day();
        
        // 每个月有两个节气
        let term1 = self.get_term_day(year, (month - 1) * 2);
        let term2 = self.get_term_day(year, (month - 1) * 2 + 1);
        
        if day == term1 {
            return Some(self.solar_terms[(month - 1) * 2].to_string());
        } else if day == term2 {
            return Some(self.solar_terms[(month - 1) * 2 + 1].to_string());
        }
        
        None
    }
    
    /// 计算节气日期
    fn get_term_day(&self, year: i32, term_index: usize) -> u32 {
        if term_index >= 24 {
            return 0;
        }
        
        // 使用节气计算公式
        // 这里使用简化公式，实际天文计算更复杂
        let century = (year - 2000) / 100;
        let d = (year - 2000) % 100;
        
        // 节气偏移值
        let c = match term_index {
            0 => 5.4055, 1 => 20.12, 2 => 3.87, 3 => 18.73, 4 => 5.63, 5 => 20.646,
            6 => 4.81, 7 => 20.1, 8 => 5.52, 9 => 21.04, 10 => 5.678, 11 => 21.37,
            12 => 7.108, 13 => 22.83, 14 => 7.5, 15 => 23.13, 16 => 7.646, 17 => 23.042,
            18 => 8.318, 19 => 23.438, 20 => 7.438, 21 => 22.36, 22 => 7.18, 23 => 21.94,
            _ => 0.0
        };
        
        // 年偏移量
        let y = match term_index {
            0 => 0.0, 1 => 0.0, 2 => 0.0, 3 => 0.0, 
            4..=9 => -0.3, 10..=13 => -0.088, 14..=16 => 0.0, 
            17..=23 => -0.3, _ => 0.0
        };
        
        // 世纪偏移量
        let l = match term_index {
            0 => 1.0, 1..=11 => 0.6, 12..=16 => 0.35, 17..=23 => 0.3, _ => 0.0
        };
        
        // 二十四节气日期计算
        let result = c + l * century as f64 + y * d as f64 / 100.0;
        
        // 四舍五入取整
        result.round() as u32
    }
    
    /// 获取生肖
    pub fn get_zodiac_animal(&self, year: i32) -> &'static str {
        let idx = (year - 4) % 12;
        self.zodiac_animals[idx as usize]
    }
    
    /// 获取日期的传统节日
    pub fn get_traditional_festival(&self, date: NaiveDate) -> Option<String> {
        let lunar_date = self.get_lunar_date(date);
        
        // 解析农历日期，提取月和日
        // 简化实现，实际需要正则解析
        
        if lunar_date.contains("正月初一") {
            return Some("春节".to_string());
        } else if lunar_date.contains("正月十五") {
            return Some("元宵节".to_string());
        } else if lunar_date.contains("五月初五") {
            return Some("端午节".to_string());
        } else if lunar_date.contains("七月初七") {
            return Some("七夕".to_string());
        } else if lunar_date.contains("八月十五") {
            return Some("中秋节".to_string());
        } else if lunar_date.contains("九月初九") {
            return Some("重阳节".to_string());
        } else if lunar_date.contains("腊月三十") || lunar_date.contains("腊月廿九") {
            return Some("除夕".to_string());
        }
        
        None
    }
}

