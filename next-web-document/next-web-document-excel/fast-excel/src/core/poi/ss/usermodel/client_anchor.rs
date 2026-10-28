use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};

/// A client anchor is attached to an Excel worksheet. It anchors against
/// absolute coordinates, a top-left cell and fixed height and width, or
/// a top-left and bottom-right cell, depending on the `AnchorType`:
/// 1. `AnchorType::DontMoveAndResize` == absolute top-left coordinates and width/height, no cell references
/// 2. `AnchorType::MoveDontResize` == fixed top-left cell reference, absolute width/height
/// 3. `AnchorType::MoveAndResize` == fixed top-left and bottom-right cell references, dynamic width/height
///
/// Note this trait only reports the current values for possibly calculated positions and sizes.
/// If the sheet row/column sizes or positions shift, this needs updating via external calculations.
pub trait ClientAnchor: Debug {
    /// Returns the column (0 based) of the first cell, or -1 if there is no top-left anchor cell.
    /// This is the case for absolute positioning `AnchorType::MoveAndResize`.
    ///
    /// # Returns
    /// 0-based column of the first cell or -1 if none.
    fn get_col1(&self) -> u16;

    /// Sets the column (0 based) of the first cell.
    ///
    /// # Arguments
    /// * `col1` - 0-based column of the first cell.
    fn set_col1(&mut self, col1: u16);

    /// Returns the column (0 based) of the second cell, or -1 if there is no bottom-right anchor cell.
    /// This is the case for absolute positioning (`AnchorType::DontMoveAndResize`)
    /// and absolute sizing (`AnchorType::MoveDontResize`).
    ///
    /// # Returns
    /// 0-based column of the second cell or -1 if none.
    fn get_col2(&self) -> u16;

    /// Sets the column (0 based) of the second cell.
    ///
    /// # Arguments
    /// * `col2` - 0-based column of the second cell.
    fn set_col2(&mut self, col2: u16);

    /// Returns the row (0 based) of the first cell, or -1 if there is no bottom-right anchor cell.
    /// This is the case for absolute positioning (`AnchorType::DontMoveAndResize`).
    ///
    /// # Returns
    /// 0-based row of the first cell or -1 if none.
    fn get_row1(&self) -> u32;

    /// Sets the row (0 based) of the first cell.
    ///
    /// # Arguments
    /// * `row1` - 0-based row of the first cell.
    fn set_row1(&mut self, row1: u32);

    /// Returns the row (0 based) of the second cell, or -1 if there is no bottom-right anchor cell.
    /// This is the case for absolute positioning (`AnchorType::DontMoveAndResize`)
    /// and absolute sizing (`AnchorType::MoveDontResize`).
    ///
    /// # Returns
    /// 0-based row of the second cell or -1 if none.
    fn get_row2(&self) -> u32;

    /// Sets the row (0 based) of the first cell.
    ///
    /// # Arguments
    /// * `row2` - 0-based row of the first cell.
    fn set_row2(&mut self, row2: u32);

    /// Returns the x coordinate within the first cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Returns
    /// The x coordinate within the first cell.
    fn get_dx1(&self) -> u32;

    /// Sets the x coordinate within the first cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Arguments
    /// * `dx1` - The x coordinate within the first cell.
    fn set_dx1(&mut self, dx1: u32);

    /// Returns the y coordinate within the first cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Returns
    /// The y coordinate within the first cell.
    fn get_dy1(&self) -> u32;

    /// Sets the y coordinate within the first cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Arguments
    /// * `dy1` - The y coordinate within the first cell.
    fn set_dy1(&mut self, dy1: u32);

    /// Returns the y coordinate within the second cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Returns
    /// The y coordinate within the second cell.
    fn get_dy2(&self) -> u32;

    /// Sets the y coordinate within the second cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Arguments
    /// * `dy2` - The y coordinate within the second cell.
    fn set_dy2(&mut self, dy2: u32);

    /// Returns the x coordinate within the second cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Returns
    /// The x coordinate within the second cell.
    fn get_dx2(&self) -> u32;

    /// Sets the x coordinate within the second cell.
    ///
    /// Note - XSSF and HSSF have a slightly different coordinate system,
    /// values in XSSF are larger by a factor of `Units::EMU_PER_PIXEL`.
    ///
    /// # Arguments
    /// * `dx2` - The x coordinate within the second cell.
    fn set_dx2(&mut self, dx2: u32);

    /// Sets the anchor type.
    ///
    /// # Arguments
    /// * `anchor_type` - The anchor type to set.
    fn set_anchor_type(&mut self, anchor_type: AnchorType);

    /// Gets the anchor type.
    ///
    /// # Returns
    /// The anchor type.
    fn get_anchor_type(&self) -> AnchorType;
}

/// Defines how a drawing anchors to cells in a worksheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorType {
    /// Move and Resize With Anchor Cells (0)
    ///
    /// Specifies that the current drawing shall move and
    /// resize to maintain its row and column anchors (i.e. the
    /// object is anchored to the actual from and to row and column).
    MoveAndResize = 0,

    /// Don't Move but do Resize With Anchor Cells (1)
    ///
    /// Specifies that the current drawing shall not move with its
    /// row and column, but should be resized. This option is not normally
    /// used, but is included for completeness.
    ///
    /// Note: Excel has no setting for this combination, nor does the ECMA standard.
    DontMoveDoResize = 1,

    /// Move With Cells but Do Not Resize (2)
    ///
    /// Specifies that the current drawing shall move with its
    /// row and column (i.e. the object is anchored to the
    /// actual from row and column), but that the size shall remain absolute.
    ///
    /// If additional rows/columns are added between the from and to locations of the drawing,
    /// the drawing shall move its to anchors as needed to maintain this same absolute size.
    MoveDontResize = 2,

    /// Do Not Move or Resize With Underlying Rows/Columns (3)
    ///
    /// Specifies that the current start and end positions shall
    /// be maintained with respect to the distances from the
    /// absolute start point of the worksheet.
    ///
    /// If additional rows/columns are added before the
    /// drawing, the drawing shall move its anchors as needed
    /// to maintain this same absolute position.
    DontMoveAndResize = 3,
}

impl Display for AnchorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorType::MoveAndResize => write!(f, "MoveAndResize"),
            AnchorType::DontMoveDoResize => write!(f, "DontMoveDoResize"),
            AnchorType::MoveDontResize => write!(f, "MoveDontResize"),
            AnchorType::DontMoveAndResize => write!(f, "DontMoveAndResize"),
        }
    }
}

impl AnchorType {
    /// Returns the AnchorType corresponding to the numeric value.
    ///
    /// # Arguments
    /// * `value` - The anchor type code.
    ///
    /// # Returns
    /// The anchor type enum.
    ///
    /// # Errors
    /// Returns `AnchorTypeError` if the value is out of range (0-3).
    pub fn by_id(value: u8) -> Result<Self, AnchorTypeError> {
        match value {
            0 => Ok(AnchorType::MoveAndResize),
            1 => Ok(AnchorType::DontMoveDoResize),
            2 => Ok(AnchorType::MoveDontResize),
            3 => Ok(AnchorType::DontMoveAndResize),
            _ => Err(AnchorTypeError::InvalidValue(value)),
        }
    }

    /// Returns the numeric value of the anchor type.
    pub fn value(&self) -> u8 {
        match self {
            AnchorType::MoveAndResize => 0,
            AnchorType::DontMoveDoResize => 1,
            AnchorType::MoveDontResize => 2,
            AnchorType::DontMoveAndResize => 3,
        }
    }
}

impl TryFrom<u8> for AnchorType {
    type Error = AnchorTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        AnchorType::by_id(value)
    }
}

impl From<AnchorType> for u8 {
    fn from(anchor_type: AnchorType) -> Self {
        anchor_type.value()
    }
}

/// Error type for invalid anchor type values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorTypeError {
    /// The provided value is not a valid anchor type (must be 0-3).
    InvalidValue(u8),
}

impl Display for AnchorTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorTypeError::InvalidValue(value) => {
                write!(
                    f,
                    "Invalid anchor type value: {}. Must be between 0 and 3.",
                    value
                )
            }
        }
    }
}
