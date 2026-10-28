/// Excel页面边距枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageMargin {
    /// Left margin, the empty space on the left of displayed worksheet data when printing
    Left,
    /// Right margin, the empty space on the right of displayed worksheet data when printing
    Right,
    /// Top margin, the empty space on the top of displayed worksheet data when printing
    Top,
    /// Bottom margin, the empty space on the bottom of displayed worksheet data when printing
    Bottom,
    /// Header margin, the empty space between the header and the top of the page when printing
    Header,
    /// Footer margin, the empty space between the footer and the bottom of the page when printing
    Footer,
}

impl PageMargin {
    pub fn value(&self) -> u16 {
        match self {
            PageMargin::Left => 0,
            PageMargin::Right => 1,
            PageMargin::Top => 2,
            PageMargin::Bottom => 3,
            PageMargin::Header => 4,
            PageMargin::Footer => 5,
        }
    }

    /// 如果值无效，返回None
    pub fn from_legacy_value(legacy_value: u16) -> Option<Self> {
        match legacy_value {
            0 => Some(PageMargin::Left),
            1 => Some(PageMargin::Right),
            2 => Some(PageMargin::Top),
            3 => Some(PageMargin::Bottom),
            4 => Some(PageMargin::Header),
            5 => Some(PageMargin::Footer),
            _ => None,
        }
    }

    /// 获取所有枚举值的向量
    pub fn all_values() -> Vec<Self> {
        vec![
            PageMargin::Left,
            PageMargin::Right,
            PageMargin::Top,
            PageMargin::Bottom,
            PageMargin::Header,
            PageMargin::Footer,
        ]
    }

    /// 检查是否是垂直边距（上、下、页眉、页脚）
    pub fn is_vertical(&self) -> bool {
        matches!(
            self,
            PageMargin::Top | PageMargin::Bottom | PageMargin::Header | PageMargin::Footer
        )
    }

    /// 检查是否是水平边距（左、右）
    pub fn is_horizontal(&self) -> bool {
        matches!(self, PageMargin::Left | PageMargin::Right)
    }

    /// 获取显示名称
    pub fn name(&self) -> &'static str {
        match self {
            PageMargin::Left => "Left",
            PageMargin::Right => "Right",
            PageMargin::Top => "Top",
            PageMargin::Bottom => "Bottom",
            PageMargin::Header => "Header",
            PageMargin::Footer => "Footer",
        }
    }
}

impl std::fmt::Display for PageMargin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PageMargin: {}", self.name())
    }
}

// 实现From<u16>以便于转换
impl TryFrom<u16> for PageMargin {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_legacy_value(value).ok_or_else(|| format!("Invalid PageMargin value: {}", value))
    }
}
