use std::fmt;

use crate::core::poi::ss::util::pane_information::PaneInformation;

/// Pane type enum matching Apache POI's PaneType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneType {
    LowerRight,
    UpperRight,
    LowerLeft,
    UpperLeft,
}

impl PaneType {
    /// Converts PaneType to its byte representation
    pub fn to_byte(&self) -> u8 {
        match self {
            PaneType::LowerRight => PaneInformation::PANE_LOWER_RIGHT,
            PaneType::UpperRight => PaneInformation::PANE_UPPER_RIGHT,
            PaneType::LowerLeft => PaneInformation::PANE_LOWER_LEFT,
            PaneType::UpperLeft => PaneInformation::PANE_UPPER_LEFT,
        }
    }

    /// Gets PaneType from byte representation
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            PaneInformation::PANE_LOWER_RIGHT => Some(PaneType::LowerRight),
            PaneInformation::PANE_UPPER_RIGHT => Some(PaneType::UpperRight),
            PaneInformation::PANE_LOWER_LEFT => Some(PaneType::LowerLeft),
            PaneInformation::PANE_UPPER_LEFT => Some(PaneType::UpperLeft),
            _ => None,
        }
    }
}

impl fmt::Display for PaneType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneType::LowerRight => write!(f, "LowerRight"),
            PaneType::UpperRight => write!(f, "UpperRight"),
            PaneType::LowerLeft => write!(f, "LowerLeft"),
            PaneType::UpperLeft => write!(f, "UpperLeft"),
        }
    }
}

// 为 PaneType 实现 FromStr 以便从字符串解析
impl std::str::FromStr for PaneType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lowerright" => Ok(PaneType::LowerRight),
            "upperright" => Ok(PaneType::UpperRight),
            "lowerleft" => Ok(PaneType::LowerLeft),
            "upperleft" => Ok(PaneType::UpperLeft),
            _ => Err(format!("Invalid pane type: {}", s)),
        }
    }
}
