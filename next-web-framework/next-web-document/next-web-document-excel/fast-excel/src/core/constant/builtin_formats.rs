use std::collections::HashMap;

use next_web_core::util::locale::Locale;

/// Excel 内置格式转换
/// 目前主要支持中文，如果不是中文，建议直接修改内置格式，未来会更好地支持国际化
///
/// 具体对应关系请参考：
/// https://docs.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.numberingformat?view=openxml-2.8.1
pub struct BuiltinFormats;

/// 格式索引类型
pub type FormatIndex = u16;

impl BuiltinFormats {
    pub const GENERAL: FormatIndex = 0;
    pub const RESERVED_PREFIX: &'static str = "reserved-";
    pub const MIN_CUSTOM_DATA_FORMAT_INDEX: FormatIndex = 82;

    /// 所有语言通用的内置格式
    pub const BUILTIN_FORMATS_ALL_LANGUAGES: [Option<&'static str>; 50] = [
        // 0
        Some("General"),
        // 1
        Some("0"),
        // 2
        Some("0.00"),
        // 3
        Some("#,##0"),
        // 4
        Some("#,##0.00"),
        // 5
        Some("\"￥\"#,##0_);(\"￥\"#,##0)"),
        // 6
        Some("\"￥\"#,##0_);[Red](\"￥\"#,##0)"),
        // 7
        Some("\"￥\"#,##0.00_);(\"￥\"#,##0.00)"),
        // 8
        Some("\"￥\"#,##0.00_);[Red](\"￥\"#,##0.00)"),
        // 9
        Some("0%"),
        // 10
        Some("0.00%"),
        // 11
        Some("0.00E+00"),
        // 12
        Some("# ?/?"),
        // 13
        Some("# ??/??"),
        // 14 - 官方文档显示 "m/d/yy"，但实际测试是 "yyyy/m/d"
        Some("yyyy/m/d"),
        // 15
        Some("d-mmm-yy"),
        // 16
        Some("d-mmm"),
        // 17
        Some("mmm-yy"),
        // 18
        Some("h:mm AM/PM"),
        // 19
        Some("h:mm:ss AM/PM"),
        // 20
        Some("h:mm"),
        // 21
        Some("h:mm:ss"),
        // 22 - 官方文档显示 "m/d/yy h:mm"，但实际测试是 "yyyy-m-d h:mm"
        Some("yyyy-m-d h:mm"),
        // 23-36 官方文档中没有找到具体对应关系
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        // 37
        Some("#,##0_);(#,##0)"),
        // 38
        Some("#,##0_);[Red](#,##0)"),
        // 39
        Some("#,##0.00_);(#,##0.00)"),
        // 40
        Some("#,##0.00_);[Red](#,##0.00)"),
        // 41
        Some("_(* #,##0_);_(* (#,##0);_(* \"-\"_);_(@_)"),
        // 42
        Some("_(\"￥\"* #,##0_);_(\"￥\"* (#,##0);_(\"￥\"* \"-\"_);_(@_)"),
        // 43
        Some("_(* #,##0.00_);_(* (#,##0.00);_(* \"-\"??_);_(@_)"),
        // 44
        Some("_(\"￥\"* #,##0.00_);_(\"￥\"* (#,##0.00);_(\"￥\"* \"-\"??_);_(@_)"),
        // 45
        Some("mm:ss"),
        // 46
        Some("[h]:mm:ss"),
        // 47
        Some("mm:ss.0"),
        // 48
        Some("##0.0E+0"),
        // 49
        Some("@"),
    ];

    /// 中文内置格式
    pub const BUILTIN_FORMATS_CN: [Option<&'static str>; 82] = [
        // 0-22 与通用格式相同
        Some("General"),
        Some("0"),
        Some("0.00"),
        Some("#,##0"),
        Some("#,##0.00"),
        Some("\"￥\"#,##0_);(\"￥\"#,##0)"),
        Some("\"￥\"#,##0_);[Red](\"￥\"#,##0)"),
        Some("\"￥\"#,##0.00_);(\"￥\"#,##0.00)"),
        Some("\"￥\"#,##0.00_);[Red](\"￥\"#,##0.00)"),
        Some("0%"),
        Some("0.00%"),
        Some("0.00E+00"),
        Some("# ?/?"),
        Some("# ??/??"),
        Some("yyyy/m/d"),
        Some("d-mmm-yy"),
        Some("d-mmm"),
        Some("mmm-yy"),
        Some("h:mm AM/PM"),
        Some("h:mm:ss AM/PM"),
        Some("h:mm"),
        Some("h:mm:ss"),
        Some("yyyy-m-d h:mm"),
        // 23-26 官方文档中没有找到具体对应关系
        None,
        None,
        None,
        None,
        // 27-36 中文特定格式
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("m\"月\"d\"日\""),
        Some("m-d-yy"),
        Some("yyyy\"年\"m\"月\"d\"日\""),
        Some("h\"时\"mm\"分\""),
        Some("h\"时\"mm\"分\"ss\"秒\""),
        Some("上午/下午h\"时\"mm\"分\""),
        Some("上午/下午h\"时\"mm\"分\"ss\"秒\""),
        Some("yyyy\"年\"m\"月\""),
        // 37-49 与通用格式相同
        Some("#,##0_);(#,##0)"),
        Some("#,##0_);[Red](#,##0)"),
        Some("#,##0.00_);(#,##0.00)"),
        Some("#,##0.00_);[Red](#,##0.00)"),
        Some("_(* #,##0_);_(* (#,##0);_(* \"-\"_);_(@_)"),
        Some("_(\"￥\"* #,##0_);_(\"￥\"* (#,##0);_(\"￥\"* \"-\"_);_(@_)"),
        Some("_(* #,##0.00_);_(* (#,##0.00);_(* \"-\"??_);_(@_)"),
        Some("_(\"￥\"* #,##0.00_);_(\"￥\"* (#,##0.00);_(\"￥\"* \"-\"??_);_(@_)"),
        Some("mm:ss"),
        Some("[h]:mm:ss"),
        Some("mm:ss.0"),
        Some("##0.0E+0"),
        Some("@"),
        // 50-81 中文特定格式
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("m\"月\"d\"日\""),
        Some("上午/下午h\"时\"mm\"分\""),
        Some("上午/下午h\"时\"mm\"分\"ss\"秒\""),
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("t0"),
        Some("t0.00"),
        Some("t#,##0"),
        Some("t#,##0.00"),
        // 63-66 官方文档中没有找到具体对应关系
        None,
        None,
        None,
        None,
        Some("t0%"),
        Some("t0.00%"),
        Some("t# ?/?"),
        Some("t# ??/??"),
        Some("ว/ด/ปปปป"),
        Some("ว-ดดด-ปป"),
        Some("ว-ดดด"),
        Some("ดดด-ปป"),
        Some("ช:นน"),
        Some("ช:นน:ทท"),
        Some("ว/ด/ปปปป ช:นน"),
        Some("นน:ทท"),
        Some("[ช]:นน:ทท"),
        Some("นน:ทท.0"),
        Some("d/m/bb"),
    ];

    /// 英文（美国）内置格式
    pub const BUILTIN_FORMATS_US: [Option<&'static str>; 82] = [
        // 0-22 与通用格式相同，但货币符号不同
        Some("General"),
        Some("0"),
        Some("0.00"),
        Some("#,##0"),
        Some("#,##0.00"),
        Some("\"$\"#,##0_);(\"$\"#,##0)"),
        Some("\"$\"#,##0_);[Red](\"$\"#,##0)"),
        Some("\"$\"#,##0.00_);(\"$\"#,##0.00)"),
        Some("\"$\"#,##0.00_);[Red](\"$\"#,##0.00)"),
        Some("0%"),
        Some("0.00%"),
        Some("0.00E+00"),
        Some("# ?/?"),
        Some("# ??/??"),
        Some("yyyy/m/d"),
        Some("d-mmm-yy"),
        Some("d-mmm"),
        Some("mmm-yy"),
        Some("h:mm AM/PM"),
        Some("h:mm:ss AM/PM"),
        Some("h:mm"),
        Some("h:mm:ss"),
        Some("yyyy-m-d h:mm"),
        // 23-26 官方文档中没有找到具体对应关系
        None,
        None,
        None,
        None,
        // 27-36 中文特定格式（在英文环境下可能不使用）
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("m\"月\"d\"日\""),
        Some("m-d-yy"),
        Some("yyyy\"年\"m\"月\"d\"日\""),
        Some("h\"时\"mm\"分\""),
        Some("h\"时\"mm\"分\"ss\"秒\""),
        Some("上午/下午h\"时\"mm\"分\""),
        Some("上午/下午h\"时\"mm\"分\"ss\"秒\""),
        Some("yyyy\"年\"m\"月\""),
        // 37-49 与通用格式相同，但货币符号不同
        Some("#,##0_);(#,##0)"),
        Some("#,##0_);[Red](#,##0)"),
        Some("#,##0.00_);(#,##0.00)"),
        Some("#,##0.00_);[Red](#,##0.00)"),
        Some("_(* #,##0_);_(* (#,##0);_(* \"-\"_);_(@_)"),
        Some("_(\"$\"* #,##0_);_(\"$\"* (#,##0);_(\"$\"* \"-\"_);_(@_)"),
        Some("_(* #,##0.00_);_(* (#,##0.00);_(* \"-\"??_);_(@_)"),
        Some("_(\"$\"* #,##0.00_);_(\"$\"* (#,##0.00);_(\"$\"* \"-\"??_);_(@_)"),
        Some("mm:ss"),
        Some("[h]:mm:ss"),
        Some("mm:ss.0"),
        Some("##0.0E+0"),
        Some("@"),
        // 50-81 中文特定格式（在英文环境下可能不使用）
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("m\"月\"d\"日\""),
        Some("上午/下午h\"时\"mm\"分\""),
        Some("上午/下午h\"时\"mm\"分\"ss\"秒\""),
        Some("yyyy\"年\"m\"月\""),
        Some("m\"月\"d\"日\""),
        Some("t0"),
        Some("t0.00"),
        Some("t#,##0"),
        Some("t#,##0.00"),
        // 63-66 官方文档中没有找到具体对应关系
        None,
        None,
        None,
        None,
        Some("t0%"),
        Some("t0.00%"),
        Some("t# ?/?"),
        Some("t# ??/??"),
        Some("ว/ด/ปปปป"),
        Some("ว-ดดด-ปป"),
        Some("ว-ดดด"),
        Some("ดดด-ปป"),
        Some("ช:นน"),
        Some("ช:นน:ทท"),
        Some("ว/ด/ปปปป ช:นน"),
        Some("นน:ทท"),
        Some("[ช]:นน:ทท"),
        Some("nน:ทท.0"),
        Some("d/m/bb"),
    ];
}

impl BuiltinFormats {
    /// 构建格式映射表
    fn build_map(builtin_formats: &[Option<&'static str>]) -> HashMap<String, FormatIndex> {
        let mut map = HashMap::with_capacity(builtin_formats.len());

        for (index, format_opt) in builtin_formats.iter().enumerate() {
            if let Some(format) = format_opt {
                map.insert(format.to_string(), index as FormatIndex);
            }
        }

        map
    }

    /// 获取中文格式映射表
    pub fn builtin_formats_map_cn() -> HashMap<String, FormatIndex> {
        Self::build_map(&Self::BUILTIN_FORMATS_CN)
    }

    /// 获取英文格式映射表
    pub fn builtin_formats_map_us() -> HashMap<String, FormatIndex> {
        Self::build_map(&Self::BUILTIN_FORMATS_US)
    }

    /// 根据区域设置选择内置格式数组
    pub fn switch_builtin_formats(locale: &Locale) -> &'static [Option<&'static str>] {
        if locale == &Locale::EnUs {
            &Self::BUILTIN_FORMATS_US
        } else {
            &Self::BUILTIN_FORMATS_CN
        }
    }

    /// 根据区域设置选择内置格式映射表
    pub fn switch_builtin_formats_map(locale: &Locale) -> HashMap<String, FormatIndex> {
        if locale == &Locale::EnUs {
            Self::builtin_formats_map_us()
        } else {
            Self::builtin_formats_map_cn()
        }
    }

    /// 获取内置格式
    pub fn get_builtin_format(
        index: Option<FormatIndex>,
        default_format: Option<&str>,
        locale: &Locale,
    ) -> Option<String> {
        let index = index?;

        // 首先检查是否是所有语言的默认值
        if (index as usize) < Self::BUILTIN_FORMATS_ALL_LANGUAGES.len() {
            if let Some(format) = Self::BUILTIN_FORMATS_ALL_LANGUAGES[index as usize] {
                return Some(format.to_string());
            }
        }

        // 其他情况下，优先使用外部提供的格式
        if let Some(default_format) = default_format {
            if !default_format.is_empty() && !default_format.starts_with(Self::RESERVED_PREFIX) {
                return Some(default_format.to_string());
            }
        }

        // 最后尝试使用内置格式
        let builtin_format = Self::switch_builtin_formats(locale);
        if (index as usize) >= builtin_format.len() {
            default_format.map(|s| s.to_string())
        } else {
            builtin_format[index as usize].map(|s| s.to_string())
        }
    }
}
