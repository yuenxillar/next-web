use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum FillPatternType {
    NoFill = 0,
    SolidForeground = 1,
    FineDots = 2,
    AltBars = 3,
    SparseDots = 4,
    ThickHorzBands = 5,
    ThickVertBands = 6,
    ThickBackwardDiag = 7,
    ThickForwardDiag = 8,
    BigSpots = 9,
    Bricks = 10,
    ThinHorzBands = 11,
    ThinVertBands = 12,
    ThinBackwardDiag = 13,
    ThinForwardDiag = 14,
    Squares = 15,
    Diamonds = 16,
    LessDots = 17,
    LeastDots = 18,
}

impl FillPatternType {
    #[inline]
    pub const fn code(self) -> u16 {
        self as u16
    }

    pub fn from_code(code: u16) -> Option<Self> {
        // 使用 match 是最快、最安全的写法（编译期优化为跳转表）
        match code {
            0 => Some(Self::NoFill),
            1 => Some(Self::SolidForeground),
            2 => Some(Self::FineDots),
            3 => Some(Self::AltBars),
            4 => Some(Self::SparseDots),
            5 => Some(Self::ThickHorzBands),
            6 => Some(Self::ThickVertBands),
            7 => Some(Self::ThickBackwardDiag),
            8 => Some(Self::ThickForwardDiag),
            9 => Some(Self::BigSpots),
            10 => Some(Self::Bricks),
            11 => Some(Self::ThinHorzBands),
            12 => Some(Self::ThinVertBands),
            13 => Some(Self::ThinBackwardDiag),
            14 => Some(Self::ThinForwardDiag),
            15 => Some(Self::Squares),
            16 => Some(Self::Diamonds),
            17 => Some(Self::LessDots),
            18 => Some(Self::LeastDots),
            _ => None,
        }
    }

    pub fn from_code_strict(code: u16) -> Self {
        Self::from_code(code).expect(&format!("Invalid FillPatternType code: {code}"))
    }

    pub const fn values() -> &'static [Self] {
        &[
            Self::NoFill,
            Self::SolidForeground,
            Self::FineDots,
            Self::AltBars,
            Self::SparseDots,
            Self::ThickHorzBands,
            Self::ThickVertBands,
            Self::ThickBackwardDiag,
            Self::ThickForwardDiag,
            Self::BigSpots,
            Self::Bricks,
            Self::ThinHorzBands,
            Self::ThinVertBands,
            Self::ThinBackwardDiag,
            Self::ThinForwardDiag,
            Self::Squares,
            Self::Diamonds,
            Self::LessDots,
            Self::LeastDots,
        ]
    }
}

impl TryFrom<u16> for FillPatternType {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_code(value).ok_or(())
    }
}

impl fmt::Display for FillPatternType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
