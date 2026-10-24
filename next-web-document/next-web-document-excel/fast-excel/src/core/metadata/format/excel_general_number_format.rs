use std::fmt::{self, Write};

use bigdecimal::RoundingMode;
use next_web_core::util::locale::Locale;
use num_traits::{Float, ToPrimitive};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Excel通用数字格式化器
/// 参考Apache POI的`ExcelGeneralNumberFormat`实现
///
/// 特点：不使用科学计数法（除非明确指定）
#[derive(Clone)]
pub struct ExcelGeneralNumberFormat {
    /// 小数符号配置（本地化）
    decimal_symbols: DecimalFormatSymbols,
    /// 整数格式化器
    integer_format: DecimalFormat,
    /// 小数格式化器
    decimal_format: DecimalFormat,
    /// 科学计数法格式化器（可选）
    scientific_format: Option<DecimalFormat>,
    /// 是否使用科学计数法
    use_scientific_format: bool,
}

impl ExcelGeneralNumberFormat {
    /// 创建新的Excel通用数字格式化器
    pub fn new(locale: Locale, use_scientific_format: bool) -> Self {
        let decimal_symbols = DecimalFormatSymbols::new(locale);

        // 支持不使用科学计数法
        let mut scientific_format = if use_scientific_format {
            Some(DecimalFormat::new("0.#####E0", decimal_symbols.clone()))
        } else {
            None
        };

        // 设置Excel风格的舍入模式
        let mut integer_format = DecimalFormat::new("#", decimal_symbols.clone());
        integer_format.set_excel_style_rounding_mode();

        let mut decimal_format = DecimalFormat::new("#.##########", decimal_symbols.clone());
        decimal_format.set_excel_style_rounding_mode();

        if let Some(sf) = scientific_format.as_mut() {
            sf.set_excel_style_rounding_mode();
        }

        ExcelGeneralNumberFormat {
            decimal_symbols,
            integer_format,
            decimal_format,
            scientific_format,
            use_scientific_format,
        }
    }

    /// 格式化数字（核心方法）
    pub fn format<T: ToPrimitive + Float>(&self, number: T) -> String {
        let mut buffer = String::new();
        self.format_to_buffer(number, &mut buffer);
        buffer
    }

    /// 格式化数字到缓冲区
    pub fn format_to_buffer<T: ToPrimitive + Float>(&self, number: T, buffer: &mut String) {
        // 处理特殊值
        if number.is_infinite() || number.is_nan() {
            self.integer_format.format_to_buffer(number, buffer);
            return;
        }

        let value = number.to_f64().unwrap_or(0.0);
        let abs_value = value.abs();

        // 判断是否使用科学计数法
        if self.use_scientific_format
            && (abs_value >= 1e11 || (abs_value <= 1e-10 && abs_value > 0.0))
        {
            if let Some(ref scientific_format) = self.scientific_format {
                scientific_format.format_to_buffer(value, buffer);
            } else {
                // 如果不允许科学计数法，使用整数格式
                self.integer_format.format_to_buffer(value, buffer);
            }
            return;
        }

        // 处理整数值或整数部分占用所有11位允许数字的情况
        if value.floor() == value || abs_value >= 1e10 {
            self.integer_format.format_to_buffer(value, buffer);
            return;
        }

        // 非整数的非科学计数法范围内的数字，格式化为"最多11个数字字符，
        // 小数点也算作一个数字字符"。我们知道有小数点，所以限制为10位数字。
        // 参考：https://support.microsoft.com/en-us/kb/65903
        let rounded = self.round_to_10_significant_figures(value);
        self.decimal_format.format_to_buffer(rounded, buffer);
    }

    /// 舍入到10位有效数字
    fn round_to_10_significant_figures(&self, value: f64) -> f64 {
        // 使用Decimal进行精确舍入
        if let Ok(decimal) = Decimal::from_f64(value) {
            // 四舍五入到10位有效数字
            let rounded = decimal.round_dp_with_strategy(
                10 - decimal.scale().max(0) as u32,
                rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            );
            rounded.to_f64().unwrap_or(value)
        } else {
            // 如果转换失败，使用原始值
            value
        }
    }

    /// 获取小数符号
    pub fn get_decimal_symbols(&self) -> &DecimalFormatSymbols {
        &self.decimal_symbols
    }

    /// 是否使用科学计数法
    pub fn uses_scientific_format(&self) -> bool {
        self.use_scientific_format
    }
}

impl fmt::Display for ExcelGeneralNumberFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExcelGeneralNumberFormat(locale={}, use_scientific={})",
            self.decimal_symbols.get_locale(),
            self.use_scientific_format
        )
    }
}

// ==================== 支持类型和结构体 ====================

/// 小数符号配置（本地化）
#[derive(Clone, Debug)]
pub struct DecimalFormatSymbols {
    locale: Locale,
    decimal_separator: char,
    grouping_separator: char,
    minus_sign: char,
    percent: char,
    per_mille: char,
    infinity: String,
    nan: String,
}

impl DecimalFormatSymbols {
    pub fn new(locale: Locale) -> Self {
        // 根据locale设置不同的符号
        match locale {
            Locale::US => DecimalFormatSymbols {
                locale: Locale::US,
                decimal_separator: '.',
                grouping_separator: ',',
                minus_sign: '-',
                percent: '%',
                per_mille: '‰',
                infinity: "∞".to_string(),
                nan: "NaN".to_string(),
            },
            Locale::FRENCH => DecimalFormatSymbols {
                locale: Locale::FRENCH,
                decimal_separator: ',',
                grouping_separator: ' ',
                minus_sign: '-',
                percent: '%',
                per_mille: '‰',
                infinity: "∞".to_string(),
                nan: "NaN".to_string(),
            },
            // 其他locale...
            _ => DecimalFormatSymbols::default(),
        }
    }

    pub fn get_locale(&self) -> &Locale {
        &self.locale
    }

    pub fn get_decimal_separator(&self) -> char {
        self.decimal_separator
    }

    pub fn get_grouping_separator(&self) -> char {
        self.grouping_separator
    }

    pub fn get_minus_sign(&self) -> char {
        self.minus_sign
    }

    pub fn get_percent(&self) -> char {
        self.percent
    }
}

impl Default for DecimalFormatSymbols {
    fn default() -> Self {
        Self::new(Locale::EnUs)
    }
}

/// 数字格式化器
#[derive(Clone, Debug)]
pub struct DecimalFormat {
    pattern: String,
    symbols: DecimalFormatSymbols,
    rounding_mode: RoundingMode,
}

impl DecimalFormat {
    pub fn new(pattern: &str, symbols: DecimalFormatSymbols) -> Self {
        DecimalFormat {
            pattern: pattern.to_string(),
            symbols,
            rounding_mode: RoundingMode::HalfUp,
        }
    }

    pub fn format_to_buffer<T: ToPrimitive>(&self, value: T, buffer: &mut String) {
        // 简化的格式化实现
        if let Some(f) = value.to_f64() {
            // 根据模式格式化
            if self.pattern.contains('E') {
                // 科学计数法
                write!(buffer, "{:.5e}", f).unwrap();
            } else if f.floor() == f {
                // 整数
                write!(buffer, "{}", f as i64).unwrap();
            } else {
                // 小数
                // 限制小数位数
                let formatted = format!("{:.10}", f)
                    .trim_end_matches('0')
                    .trim_end_matches('.');
                buffer.push_str(formatted);
            }
        }
    }

    pub fn set_excel_style_rounding_mode(&mut self) {
        self.rounding_mode = RoundingMode::HalfUp;
    }
}
