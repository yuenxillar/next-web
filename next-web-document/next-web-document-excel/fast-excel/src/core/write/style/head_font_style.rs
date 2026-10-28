#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadFontStyle {
    pub font_name: Option<String>,
    /// 字号（点数），-1 表示未设置
    pub font_height_in_points: i16,
    /// 是否斜体
    pub italic: Option<bool>,
    /// 是否删除线
    pub strikeout: Option<bool>,
    /// 字体颜色索引（对应 IndexedColors），-1 表示未设置
    pub color: i16,
    /// 上标/下标：0=无，1=上标，2=下标，-1=未设置
    pub type_offset: i16,
    /// 下划线样式，-1=未设置
    pub underline: i8,
    /// 字符集，-1=未设置
    pub charset: i32,
    /// 是否加粗
    pub bold: Option<bool>,
}

impl Default for HeadFontStyle {
    #[inline]
    fn default() -> Self {
        Self {
            font_name: None,
            font_height_in_points: -1,
            italic: None,
            strikeout: None,
            color: -1,
            type_offset: -1,
            underline: -1,
            charset: -1,
            bold: None,
        }
    }
}

impl HeadFontStyle {
    #[inline]
    pub fn has_font_name(&self) -> bool {
        !self
            .font_name
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true)
    }

    /// 安全获取字号（>=0 表示已设置）
    #[inline]
    pub fn is_font_height_set(&self) -> bool {
        self.font_height_in_points >= 0
    }

    /// 安全获取颜色
    #[inline]
    pub fn is_color_set(&self) -> bool {
        self.color >= 0
    }

    /// 安全获取 type_offset
    #[inline]
    pub fn is_type_offset_set(&self) -> bool {
        self.type_offset >= 0
    }

    /// 安全获取 underline
    #[inline]
    pub fn is_underline_set(&self) -> bool {
        self.underline >= 0
    }

    /// 安全获取 charset
    #[inline]
    pub fn is_charset_set(&self) -> bool {
        self.charset >= 0
    }
}
