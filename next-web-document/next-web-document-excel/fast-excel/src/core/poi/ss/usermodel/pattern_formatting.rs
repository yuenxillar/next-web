use std::fmt::Debug;

use crate::core::poi::ss::usermodel::color::Color;

pub mod pattern_formatting_constants {
    /// No background
    pub const NO_FILL: u16 = 0;
    /// Solidly filled
    pub const SOLID_FOREGROUND: u16 = 1;
    /// Small fine dots
    pub const FINE_DOTS: u16 = 2;
    /// Wide dots
    pub const ALT_BARS: u16 = 3;
    /// Sparse dots
    pub const SPARSE_DOTS: u16 = 4;
    /// Thick horizontal bands
    pub const THICK_HORZ_BANDS: u16 = 5;
    /// Thick vertical bands
    pub const THICK_VERT_BANDS: u16 = 6;
    /// Thick backward facing diagonals
    pub const THICK_BACKWARD_DIAG: u16 = 7;
    /// Thick forward facing diagonals
    pub const THICK_FORWARD_DIAG: u16 = 8;
    /// Large spots
    pub const BIG_SPOTS: u16 = 9;
    /// Brick-like layout
    pub const BRICKS: u16 = 10;
    /// Thin horizontal bands
    pub const THIN_HORZ_BANDS: u16 = 11;
    /// Thin vertical bands
    pub const THIN_VERT_BANDS: u16 = 12;
    /// Thin backward diagonal
    pub const THIN_BACKWARD_DIAG: u16 = 13;
    /// Thin forward diagonal
    pub const THIN_FORWARD_DIAG: u16 = 14;
    /// Squares
    pub const SQUARES: u16 = 15;
    /// Diamonds
    pub const DIAMONDS: u16 = 16;
    /// Less Dots
    pub const LESS_DOTS: u16 = 17;
    /// Least Dots
    pub const LEAST_DOTS: u16 = 18;
}

/// Pattern formatting interface for conditional formatting and cell styles.
pub trait PatternFormatting: Debug {
    /// Get the fill background color index.
    ///
    /// # Returns
    /// Background color index.
    fn get_fill_background_color(&self) -> u16;

    /// Get the fill foreground color index.
    ///
    /// # Returns
    /// Foreground color index.
    fn get_fill_foreground_color(&self) -> u16;

    /// Get the fill background color object.
    ///
    /// # Returns
    /// Background color object, or `None` if not set.
    fn get_fill_background_color_color(&self) -> Option<&dyn Color>;

    /// Get the fill foreground color object.
    ///
    /// # Returns
    /// Foreground color object, or `None` if not set.
    fn get_fill_foreground_color_color(&self) -> Option<&dyn Color>;

    /// Get the fill pattern.
    ///
    /// # Returns
    /// Fill pattern code (one of the pattern constants).
    fn get_fill_pattern(&self) -> u16;

    /// Set the fill background color by index.
    ///
    /// # Arguments
    /// * `bg` - Background color index.
    fn set_fill_background_color_index(&mut self, bg: u16);

    /// Set the fill foreground color by index.
    ///
    /// # Arguments
    /// * `fg` - Foreground color index.
    fn set_fill_foreground_color_index(&mut self, fg: u16);

    /// Set the fill background color by object.
    ///
    /// # Arguments
    /// * `bg` - Background color object.
    fn set_fill_background_color(&mut self, bg: Box<dyn Color>);

    /// Set the fill foreground color by object.
    ///
    /// # Arguments
    /// * `fg` - Foreground color object.
    fn set_fill_foreground_color(&mut self, fg: Box<dyn Color>);

    /// Set the fill pattern.
    ///
    /// # Arguments
    /// * `fp` - Fill pattern code (one of the pattern constants).
    fn set_fill_pattern(&mut self, fp: u16);
}
