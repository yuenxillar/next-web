use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum VerticalAlignment {
    Top = 0,
    Center = 1,
    Bottom = 2,
    Justify = 3,
    Distributed = 4,
}

impl VerticalAlignment {
    #[inline]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// 安全版：从 code 恢复枚举（非法值返回 None）
    #[inline]
    pub const fn from_code(code: u16) -> Option<Self> {
        if code < 5 {
            Some(unsafe { std::mem::transmute(code) })
        } else {
            None
        }
    }

    #[inline]
    pub fn from_code_strict(code: u16) -> Self {
        Self::from_code(code).unwrap_or_else(|| panic!("Invalid VerticalAlignment code: {code}"))
    }

    pub const fn values() -> &'static [Self] {
        &[
            Self::Top,
            Self::Center,
            Self::Bottom,
            Self::Justify,
            Self::Distributed,
        ]
    }
}

impl TryFrom<u16> for VerticalAlignment {
    type Error = ();

    #[inline]
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_code(value).ok_or(())
    }
}

impl From<VerticalAlignment> for u16 {
    #[inline]
    fn from(val: VerticalAlignment) -> Self {
        val.code()
    }
}

impl fmt::Display for VerticalAlignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
