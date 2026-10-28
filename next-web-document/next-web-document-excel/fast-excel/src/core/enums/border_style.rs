use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum BorderStyle {
    None = 0,
    Thin = 1,
    Medium = 2,
    Dashed = 3,
    Dotted = 4,
    Thick = 5,
    Double = 6,
    Hair = 7,
    MediumDashed = 8,
    DashDot = 9,
    MediumDashDot = 10,
    DashDotDot = 11,
    MediumDashDotDot = 12,
    SlantedDashDot = 13,
}

impl BorderStyle {
    #[inline]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// 安全版：从 POI 的 short code 恢复枚举（非法值返回 None）
    #[inline]
    pub const fn from_code(code: u16) -> Option<Self> {
        if code <= 13 {
            Some(unsafe { std::mem::transmute(code) })
        } else {
            None
        }
    }

    #[inline]
    pub fn value_of(code: u16) -> Self {
        Self::from_code(code).unwrap_or_else(|| panic!("Invalid BorderStyle code: {code}"))
    }

    pub const fn values() -> &'static [Self] {
        &[
            Self::None,
            Self::Thin,
            Self::Medium,
            Self::Dashed,
            Self::Dotted,
            Self::Thick,
            Self::Double,
            Self::Hair,
            Self::MediumDashed,
            Self::DashDot,
            Self::MediumDashDot,
            Self::DashDotDot,
            Self::MediumDashDotDot,
            Self::SlantedDashDot,
        ]
    }
}

impl TryFrom<u16> for BorderStyle {
    type Error = ();

    #[inline]
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_code(value).ok_or(())
    }
}

impl From<BorderStyle> for u16 {
    #[inline]
    fn from(val: BorderStyle) -> Self {
        val.code()
    }
}

impl fmt::Display for BorderStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
