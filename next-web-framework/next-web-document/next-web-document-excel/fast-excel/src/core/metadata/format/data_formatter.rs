use chrono::{DateTime, FixedOffset, NaiveDate};
use next_web_core::util::locale::Locale;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

// 全局正则表达式（线程安全）
static NUM_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[0#]+").unwrap());
static DAYS_AS_TEXT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(d{3,})").unwrap());
static AM_PM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(([AP])[M/P]*)|(([上下])[午/下]*)").unwrap());
static COLOR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\[(BLACK|BLUE|CYAN|GREEN|MAGENTA|RED|WHITE|YELLOW|COLOR\s*\d+|COLOR\s*[0-5]\d|DBNum[123]|\$-?[0-9A-Z]+)\]").unwrap()
});
static FRACTION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:([#\d]+)\s+)?(#+)\s*/\s*([#\d]+)").unwrap());
static FRACTION_STRIPPER: Lazy<Regex> = Lazy::new(|| Regex::new(r#""[^"]*"|[^ ?#\d/]+"#).unwrap());
static LOCALE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\$[^-\]]*-?[0-9A-Z]+\]").unwrap());
static E_NOTATION_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"E(\d)").unwrap());

#[derive(Clone)]
pub struct DataFormatter {
    invalid_date_time_string: String,

    /// The decimal symbols of the locale used for formatting values.
    decimal_format: Option<String>,

    /// The date symbols of the locale used for formatting values.
    date_format: Option<String>,

    /// A default format to use when a number pattern cannot be parsed.
    // defaultNumFormat:
    locale: Locale,
    use_1904_windowing: bool,
    use_scientific_format: bool,
    formats_cache: HashMap<String, Box<dyn FormatTrait>>,
}

pub trait FormatTrait {
    fn format(&self, value: f64) -> String;
    fn format_date(&self, date: DateTime<FixedOffset>) -> String;
}

impl DataFormatter {
    pub fn new(
        use_1904_windowing: Option<bool>,
        locale: Locale,
        use_scientific_format: bool,
    ) -> Self {
        let invalid_date_time_string = "#".repeat(255);
        Self {
            use_1904_windowing: use_1904_windowing.unwrap_or_default(),
            locale,
            use_scientific_format: if use_1904_windowing.is_none() {
                false
            } else {
                use_scientific_format
            },
            formats_cache: Default::default(),
            invalid_date_time_string,
        }
    }

    pub fn format_raw(&self, value: f64, format_index: i16, format_string: &str) -> String {
        if DateUtil::is_date_format(format_index, format_string) {
            let date = DateUtil::excel_to_date(value, self.use_1904_windowing);
            return self.format_date(value, format_string, date);
        }

        self.format_number(value, format_string)
    }

    fn format_date(
        &self,
        excel_serial: f64,
        format_str: &str,
        date: DateTime<FixedOffset>,
    ) -> String {
        let format = self.get_or_create_format(format_str.to_string());
        format.format_date(date)
    }

    fn format_number(&self, value: f64, format_str: &str) -> String {
        let format = self.get_or_create_format(format_str.to_string());
        let mut s = format.format(value);
        // 修复科学计数法中的 E+n → En
        s = E_NOTATION_PATTERN.replace_all(&s, "E$1").to_string();
        s
    }

    fn get_or_create_format(&self, mut format_str: String) -> Box<dyn FormatTrait + Send + Sync> {
        if let Some(fmt) = self.formats_cache.get(&format_str) {
            return fmt.clone_box();
        }

        let cleaned = self.preprocess_format_string(&format_str);
        let fmt: Box<dyn FormatTrait + Send + Sync> = if cleaned == "General" || cleaned == "@" {
            Box::new(GeneralFormat)
        } else if DateUtil::is_date_format(-1, &cleaned) {
            Box::new(DateFormat::new(&cleaned))
        } else if cleaned.contains('/') && (cleaned.contains("#/") || cleaned.contains("?/?")) {
            Box::new(FractionFormat::new(&cleaned))
        } else if NUM_PATTERN.is_match(&cleaned) {
            Box::new(NumberFormat::new(&cleaned))
        } else if let Some(special) = SpecialFormat::from_str(&format_str) {
            Box::new(special)
        } else {
            Box::new(GeneralFormat)
        };

        let mut cache = Arc::try_unwrap(self.formats_cache.clone()).unwrap_or_default();
        cache.insert(format_str, fmt.clone_box());
        self.formats_cache = Arc::new(cache);
        fmt
    }

    fn preprocess_format_string<'a>(&self, s: &'a str) -> String {
        let mut s = s.to_string();

        // 去除颜色
        s = COLOR_PATTERN.replace_all(&s, "").to_string();

        // 去除区域货币符号 [$-409] 等
        s = LOCALE_PATTERN.replace_all(&s, "").to_string();

        // 去除多余引号和转义
        s = s
            .replace("\\-", "-")
            .replace("\\ ", " ")
            .replace("\\.", ".")
            .replace("\\/", "/")
            .replace("\"\"", "'")
            .replace("\"", "");

        s
    }
}

// ==================== 日期工具 ====================
pub struct DateUtil;
impl DateUtil {
    pub fn is_date_format(_format_index: i16, format_str: &str) -> bool {
        let lower = format_str.to_lowercase();
        lower.contains('y')
            || lower.contains('m')
            || lower.contains('d')
            || lower.contains('h')
            || lower.contains('s')
    }

    pub fn excel_to_date(excel_serial: f64, use_1904: bool) -> DateTime<FixedOffset> {
        let days = excel_serial.trunc() as i64;
        let time = excel_serial.fract();

        let base_date = if use_1904 {
            NaiveDate::from_ymd_opt(1904, 1, 1).unwrap()
        } else {
            // Excel 错误：1900 是闰年
            NaiveDate::from_ymd_opt(1899, 12, 30).unwrap()
        };

        let date = base_date + chrono::Duration::days(days);
        let seconds_in_day = (time * 86400.0).round() as i64;
        let dt = date.and_hms_opt(0, 0, 0).unwrap() + chrono::Duration::seconds(seconds_in_day);

        DateTime::from_utc(dt.naive_utc(), FixedOffset::east_opt(0).unwrap())
    }
}

// ==================== 特殊格式 ====================
#[derive(Clone)]
enum SpecialFormat {
    SSN,
    ZipPlus4,
    Phone,
}

impl SpecialFormat {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "00000-0000" | "00000\\-0000" => Some(SpecialFormat::ZipPlus4),
            "000-00-0000" | "000\\-00-0000" => Some(SpecialFormat::SSN),
            s if s.contains("###-####") || s.contains("(###)") => Some(SpecialFormat::Phone),
            _ => None,
        }
    }
}

impl FormatTrait for SpecialFormat {
    fn format(&self, value: f64) -> String {
        let num = value as u64;
        let s = format!("{:09}", num); // 至少9位
        match self {
            SpecialFormat::SSN => format!("{}-{}-{}", &s[0..3], &s[3..5], &s[5..9]),
            SpecialFormat::ZipPlus4 => format!("{}-{}", &s[0..5], &s[5..9]),
            SpecialFormat::Phone => {
                let s = format!("{:010}", num);
                let area = &s[0..3];
                let exch = &s[3..6];
                let num = &s[6..10];
                if area == "000" {
                    num.to_string()
                } else {
                    format!("({}) {}-{}", area, exch, num)
                }
            }
        }
    }

    fn format_date(&self, _: DateTime<FixedOffset>) -> String {
        String::new()
    }
}

// ==================== 通用格式 ====================
struct GeneralFormat;
impl FormatTrait for GeneralFormat {
    fn format(&self, value: f64) -> String {
        if value.fract() == 0.0 {
            format!("{}", value as i64)
        } else if value.abs() >= 1e11 || (value.abs() < 1e-4 && value != 0.0) {
            format!("{:.2E}", value).replace("E+", "E")
        } else {
            let s = format!("{:.15}", value);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }

    fn format_date(&self, date: DateTime<FixedOffset>) -> String {
        date.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

// ==================== 数字格式 ====================
struct NumberFormat {
    pattern: String,
}
impl NumberFormat {
    fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }
}
impl FormatTrait for NumberFormat {
    fn format(&self, value: f64) -> String {
        // 简化实现，实际可用 ryu 或 rust_decimal
        format!("{:.2}", value)
    }
    fn format_date(&self, _: DateTime<FixedOffset>) -> String {
        String::new()
    }
}

// ==================== 分数格式 ====================
struct FractionFormat {
    whole: String,
    denom: String,
}
impl FractionFormat {
    fn new(s: &str) -> Self {
        let s = s.replace('?', "#");
        let cleaned = FRACTION_STRIPPER.replace_all(&s, " ").replace(" +", " ");
        if let Some(caps) = FRACTION_PATTERN.captures(&cleaned) {
            let whole = caps.get(1).map(|m| m.as_str()).unwrap_or("#").to_string();
            let denom = caps.get(3).map(|m| m.as_str()).unwrap_or("##").to_string();
            Self { whole, denom }
        } else {
            Self {
                whole: "#".to_string(),
                denom: "##".to_string(),
            }
        }
    }
}
impl FormatTrait for FractionFormat {
    fn format(&self, value: f64) -> String {
        // 简化：转为最简分数
        let sign = if value < 0.0 { "-" } else { "" };
        let abs = value.abs();
        let whole = abs.floor() as i32;
        let frac = abs - whole as f64;
        if frac < 1e-10 {
            return format!("{}{}", sign, whole);
        }
        // 简单近似
        format!("{}{} {}/8", sign, whole, (frac * 8.0).round())
    }
    fn format_date(&self, _: DateTime<FixedOffset>) -> String {
        String::new()
    }
}

// ==================== 日期格式（简化） ====================
struct DateFormat {
    pattern: String,
}
impl DateFormat {
    fn new(pattern: &str) -> Self {
        let mut p = pattern.to_string();
        p = p
            .replace("yyyy", "%Y")
            .replace("yy", "%y")
            .replace("mmmm", "%B")
            .replace("mmm", "%b")
            .replace("mm", "%m")
            .replace("m", "%-m")
            .replace("dd", "%d")
            .replace("d", "%-d")
            .replace("hh", "%H")
            .replace("h", "%I")
            .replace("HH", "%H")
            .replace("H", "%k")
            .replace("ss", "%S")
            .replace("am/pm", "%p");
        Self { pattern: p }
    }
}
impl FormatTrait for DateFormat {
    fn format(&self, _: f64) -> String {
        String::new()
    }
    fn format_date(&self, date: DateTime<FixedOffset>) -> String {
        date.format(&self.pattern).to_string()
    }
}

// 扩展 trait 用于 clone Box
trait CloneBoxDynFormat {
    fn clone_box(&self) -> Box<dyn FormatTrait + Send + Sync>;
}
impl<T: 'static + FormatTrait + Send + Sync + Clone> CloneBoxDynFormat for T {
    fn clone_box(&self) -> Box<dyn FormatTrait + Send + Sync> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn FormatTrait + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
