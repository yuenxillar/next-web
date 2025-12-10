use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum HorizontalAlignment {
    General = 0,
    Left = 1,
    Center = 2,
    Right = 3,
    Fill = 4,
    Justify = 5,
    CenterSelection = 6,
    Distributed = 7,
}

impl HorizontalAlignment {
    #[inline]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// 从 POI 的 short code 恢复枚举（安全版，返回 Option）
    #[inline]
    pub const fn from_code(code: u16) -> Option<Self> {
        if code < 8 {
            // 编译期安全：所有合法值都在 0..8
            Some(unsafe { std::mem::transmute(code) })
        } else {
            None
        }
    }

    #[inline]
    pub fn from_code_strict(code: u16) -> Self {
        Self::from_code(code).unwrap_or_else(|| panic!("Invalid HorizontalAlignment code: {code}"))
    }

    pub const fn values() -> &'static [Self] {
        &[
            Self::General,
            Self::Left,
            Self::Center,
            Self::Right,
            Self::Fill,
            Self::Justify,
            Self::CenterSelection,
            Self::Distributed,
        ]
    }
}

impl TryFrom<u16> for HorizontalAlignment {
    type Error = ();

    #[inline]
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_code(value).ok_or(())
    }
}

impl From<HorizontalAlignment> for u16 {
    #[inline]
    fn from(val: HorizontalAlignment) -> Self {
        val.code()
    }
}

impl fmt::Display for HorizontalAlignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
