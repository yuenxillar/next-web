use std::fmt::Debug;

use crate::core::poi::ss::usermodel::color::Color;

/// High level representation for Font Formatting component
/// of Conditional Formatting settings
pub trait FontFormatting: Debug {
    // TODO: refactor and unify Font & FontFormatting in POI 5.0.0

    /// Get the type of super or subscript for the font.
    ///
    /// # Returns
    /// Super or subscript option.
    fn get_escapement_type(&self) -> u16;

    /// Set the escapement type for the font.
    ///
    /// # Arguments
    /// * `escapement_type` - super or subscript option.
    fn set_escapement_type(&mut self, escapement_type: u16);

    /// Get the font colour index.
    ///
    /// # Returns
    /// Font colour index, or 0 if not indexed (XSSF only).
    fn get_font_color_index(&self) -> u16;

    /// Set the indexed colour to use.
    ///
    /// # Arguments
    /// * `color` - font colour index.
    fn set_font_color_index(&mut self, color: u16);

    /// Get the colour of the font.
    ///
    /// # Returns
    /// The colour of the font, or `None` if no colour applied.
    fn get_font_color(&self) -> Option<&dyn Color>;

    /// Set the colour to use.
    ///
    /// # Arguments
    /// * `color` - font colour to use.
    fn set_font_color(&mut self, color: Box<dyn Color>);

    /// Get the height of the font in 1/20th point units.
    ///
    /// # Returns
    /// Font height (in points/20); or -1 if not modified.
    fn get_font_height(&self) -> i32;

    /// Set the height of the font in 1/20th point units.
    ///
    /// # Arguments
    /// * `height` - the height in twips (in points/20).
    fn set_font_height(&mut self, height: i32);

    /// Get the type of underlining for the font.
    ///
    /// # Returns
    /// Font underlining type.
    fn get_underline_type(&self) -> u16;

    /// Set the type of underlining type for the font.
    ///
    /// # Arguments
    /// * `underline_type` - super or subscript option.
    fn set_underline_type(&mut self, underline_type: u16);

    /// Get whether the font weight is set to bold or not.
    ///
    /// # Returns
    /// `true` if the font is bold, `false` otherwise.
    fn is_bold(&self) -> bool;

    /// Check if font style was set to italic.
    ///
    /// # Returns
    /// `true` if font style was set to italic.
    fn is_italic(&self) -> bool;

    /// Check if font strikeout is on.
    ///
    /// # Returns
    /// `true` if font strikeout is on.
    fn is_struckout(&self) -> bool;

    /// Set font style options.
    ///
    /// # Arguments
    /// * `italic` - if `true`, set posture style to italic, otherwise to normal.
    /// * `bold` - if `true`, set font weight to bold, otherwise to normal.
    fn set_font_style(&mut self, italic: bool, bold: bool);

    /// Set font style options to default values (non-italic, non-bold).
    fn reset_font_style(&mut self);
}
