use std::fmt;

use crate::core::poi::ss::usermodel::pane_type::PaneType;

/// Holds information regarding a split plane or freeze plane for a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneInformation {
    /// Vertical position of the split
    x: u16,
    /// Horizontal position of the split
    y: u16,
    /// Top row in the BOTTOM pane for horizontal splits
    top_row: u16,
    /// Left column in the RIGHT pane for vertical splits
    left_column: u16,
    /// Active pane identifier
    active_pane: u8,
    /// Whether this is a freeze pane (true) or split pane (false)
    frozen: bool,
}

impl PaneInformation {
    /// Constant for active pane being the lower right
    pub const PANE_LOWER_RIGHT: u8 = 0;
    /// Constant for active pane being the upper right
    pub const PANE_UPPER_RIGHT: u8 = 1;
    /// Constant for active pane being the lower left
    pub const PANE_LOWER_LEFT: u8 = 2;
    /// Constant for active pane being the upper left
    pub const PANE_UPPER_LEFT: u8 = 3;

    /// Creates a new PaneInformation instance
    pub fn new(x: u16, y: u16, top: u16, left: u16, active: u8, frozen: bool) -> Self {
        PaneInformation {
            x,
            y,
            top_row: top,
            left_column: left,
            active_pane: active,
            frozen,
        }
    }

    /// Returns the vertical position of the split.
    ///
    /// - Returns 0 if there is no vertical split
    /// - For a freeze pane: the number of columns in the TOP pane
    /// - For a split plane: the position of the split in 1/20th of a point
    pub fn get_vertical_split_position(&self) -> u16 {
        self.x
    }

    /// Returns the horizontal position of the split.
    ///
    /// - Returns 0 if there is no horizontal split
    /// - For a freeze pane: the number of rows in the LEFT pane
    /// - For a split plane: the position of the split in 1/20th of a point
    pub fn get_horizontal_split_position(&self) -> u16 {
        self.y
    }

    /// For a horizontal split returns the top row in the BOTTOM pane.
    ///
    /// - Returns 0 if there is no horizontal split
    /// - Otherwise returns the top row of the bottom pane
    pub fn get_horizontal_split_top_row(&self) -> u16 {
        self.top_row
    }

    /// For a vertical split returns the left column in the RIGHT pane.
    ///
    /// - Returns 0 if there is no vertical split
    /// - Otherwise returns the left column in the RIGHT pane
    pub fn get_vertical_split_left_column(&self) -> u16 {
        self.left_column
    }

    /// Returns the active pane.
    ///
    /// # Returns
    /// One of the pane constants:
    /// - `PANE_LOWER_RIGHT`
    /// - `PANE_UPPER_RIGHT`
    /// - `PANE_LOWER_LEFT`
    /// - `PANE_UPPER_LEFT`
    pub fn get_active_pane(&self) -> u8 {
        self.active_pane
    }

    /// Returns the active pane type.
    ///
    /// # Returns
    /// - `Some(PaneType)` for valid pane identifiers
    /// - `None` if no active pane type is set or for invalid identifiers
    pub fn get_active_pane_type(&self) -> Option<PaneType> {
        match self.active_pane {
            Self::PANE_LOWER_RIGHT => Some(PaneType::LowerRight),
            Self::PANE_UPPER_RIGHT => Some(PaneType::UpperRight),
            Self::PANE_LOWER_LEFT => Some(PaneType::LowerLeft),
            Self::PANE_UPPER_LEFT => Some(PaneType::UpperLeft),
            _ => None,
        }
    }

    /// Returns true if this is a Freeze pane, false if it is a split pane.
    pub fn is_freeze_pane(&self) -> bool {
        self.frozen
    }
}

impl fmt::Display for PaneInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PaneInformation{{x={}, y={}, topRow={}, leftColumn={}, activePane={}, frozen={}}}",
            self.x, self.y, self.top_row, self.left_column, self.active_pane, self.frozen
        )
    }
}
