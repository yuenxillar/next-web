use std::convert::TryFrom;

/// The enumeration value indicating the line style of a border in a cell,
/// i.e., whether it is bordered dash dot, dash dot dot, dashed, dotted, double, hair, medium,
/// medium dash dot, medium dash dot dot, medium dashed, none, slant dash dot, thick or thin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorderStyle {
    /// No border (default)
    None = 0x0,

    /// Thin border
    Thin = 0x1,

    /// Medium border
    Medium = 0x2,

    /// dash border
    Dashed = 0x3,

    /// dot border
    Dotted = 0x4,

    /// Thick border
    Thick = 0x5,

    /// double-line border
    Double = 0x6,

    /// hair-line border
    Hair = 0x7,

    /// Medium dashed border
    MediumDashed = 0x8,

    /// dash-dot border
    DashDot = 0x9,

    /// medium dash-dot border
    MediumDashDot = 0xA,

    /// dash-dot-dot border
    DashDotDot = 0xB,

    /// medium dash-dot-dot border
    MediumDashDotDot = 0xC,

    /// slanted dash-dot border
    SlantedDashDot = 0xD,
}

impl BorderStyle {
    /// Get the numeric code for this border style.
    ///
    /// # Returns
    /// The numeric code.
    pub fn get_code(&self) -> u8 {
        *self as u8
    }

    /// Convert a numeric code to a BorderStyle.
    ///
    /// # Arguments
    /// * `code` - The numeric code.
    ///
    /// # Returns
    /// The corresponding BorderStyle, or `None` if the code is invalid.
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0x0 => Some(BorderStyle::None),
            0x1 => Some(BorderStyle::Thin),
            0x2 => Some(BorderStyle::Medium),
            0x3 => Some(BorderStyle::Dashed),
            0x4 => Some(BorderStyle::Dotted),
            0x5 => Some(BorderStyle::Thick),
            0x6 => Some(BorderStyle::Double),
            0x7 => Some(BorderStyle::Hair),
            0x8 => Some(BorderStyle::MediumDashed),
            0x9 => Some(BorderStyle::DashDot),
            0xA => Some(BorderStyle::MediumDashDot),
            0xB => Some(BorderStyle::DashDotDot),
            0xC => Some(BorderStyle::MediumDashDotDot),
            0xD => Some(BorderStyle::SlantedDashDot),
            _ => None,
        }
    }

    /// Check if this border style represents "no border".
    ///
    /// # Returns
    /// `true` if this is BorderStyle::None, `false` otherwise.
    pub fn is_none(&self) -> bool {
        matches!(self, BorderStyle::None)
    }

    /// Check if this border style represents a solid line (not dashed/dotted).
    ///
    /// # Returns
    /// `true` if this is a solid border style, `false` otherwise.
    pub fn is_solid(&self) -> bool {
        matches!(
            self,
            BorderStyle::Thin
                | BorderStyle::Medium
                | BorderStyle::Thick
                | BorderStyle::Double
                | BorderStyle::Hair
        )
    }

    /// Check if this border style represents a dashed or dotted line.
    ///
    /// # Returns
    /// `true` if this is a dashed/dotted border style, `false` otherwise.
    pub fn is_dashed_or_dotted(&self) -> bool {
        matches!(
            self,
            BorderStyle::Dashed
                | BorderStyle::Dotted
                | BorderStyle::MediumDashed
                | BorderStyle::DashDot
                | BorderStyle::MediumDashDot
                | BorderStyle::DashDotDot
                | BorderStyle::MediumDashDotDot
                | BorderStyle::SlantedDashDot
        )
    }
}

impl TryFrom<u8> for BorderStyle {
    type Error = &'static str;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        BorderStyle::from_code(code).ok_or("Invalid border style code")
    }
}

impl From<BorderStyle> for u8 {
    fn from(style: BorderStyle) -> Self {
        style.get_code()
    }
}

impl std::fmt::Display for BorderStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            BorderStyle::None => "None",
            BorderStyle::Thin => "Thin",
            BorderStyle::Medium => "Medium",
            BorderStyle::Dashed => "Dashed",
            BorderStyle::Dotted => "Dotted",
            BorderStyle::Thick => "Thick",
            BorderStyle::Double => "Double",
            BorderStyle::Hair => "Hair",
            BorderStyle::MediumDashed => "MediumDashed",
            BorderStyle::DashDot => "DashDot",
            BorderStyle::MediumDashDot => "MediumDashDot",
            BorderStyle::DashDotDot => "DashDotDot",
            BorderStyle::MediumDashDotDot => "MediumDashDotDot",
            BorderStyle::SlantedDashDot => "SlantedDashDot",
        };
        write!(f, "{}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_style_conversions() {
        // Test code to enum conversion
        assert_eq!(BorderStyle::from_code(0x0), Some(BorderStyle::None));
        assert_eq!(BorderStyle::from_code(0x1), Some(BorderStyle::Thin));
        assert_eq!(
            BorderStyle::from_code(0xD),
            Some(BorderStyle::SlantedDashDot)
        );
        assert_eq!(BorderStyle::from_code(0xE), None); // Invalid code

        // Test enum to code conversion
        assert_eq!(BorderStyle::Thin.get_code(), 0x1);
        assert_eq!(BorderStyle::MediumDashDotDot.get_code(), 0xC);

        // Test TryFrom implementation
        assert_eq!(BorderStyle::try_from(0x2), Ok(BorderStyle::Medium));
        assert!(BorderStyle::try_from(0xFF).is_err());

        // Test From implementation
        let code: u8 = BorderStyle::Double.into();
        assert_eq!(code, 0x6);

        // Test utility methods
        assert!(BorderStyle::None.is_none());
        assert!(!BorderStyle::Thin.is_none());

        assert!(BorderStyle::Thin.is_solid());
        assert!(!BorderStyle::Dashed.is_solid());

        assert!(BorderStyle::Dashed.is_dashed_or_dotted());
        assert!(!BorderStyle::Thin.is_dashed_or_dotted());

        // Test Display implementation
        assert_eq!(BorderStyle::Thick.to_string(), "Thick");
        assert_eq!(BorderStyle::MediumDashDot.to_string(), "MediumDashDot");
    }
}
