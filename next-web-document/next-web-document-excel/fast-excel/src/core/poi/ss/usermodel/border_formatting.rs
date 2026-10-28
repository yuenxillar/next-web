use std::fmt::Debug;

use crate::core::poi::ss::usermodel::{border_style::BorderStyle, color::Color};

/// High level representation for Border Formatting component
/// of Conditional Formatting settings
pub trait BorderFormatting: Debug {
    /// Get the bottom border style.
    ///

    fn get_border_bottom(&self) -> BorderStyle;

    /// Get the diagonal border style.
    ///

    fn get_border_diagonal(&self) -> BorderStyle;

    /// Get the left border style.
    ///

    fn get_border_left(&self) -> BorderStyle;

    /// Get the right border style.
    ///

    fn get_border_right(&self) -> BorderStyle;

    /// Get the top border style.
    ///

    fn get_border_top(&self) -> BorderStyle;

    /// Get the vertical border style.
    ///
    /// Only valid for range borders, such as table styles.
    ///

    fn get_border_vertical(&self) -> BorderStyle;

    /// Get the horizontal border style.
    ///
    /// Only valid for range borders, such as table styles.
    ///

    fn get_border_horizontal(&self) -> BorderStyle;

    /// Get the bottom border color index.
    fn get_bottom_border_color(&self) -> u16;

    /// Get the bottom border color object.
    fn get_bottom_border_color_color(&self) -> Option<&dyn Color>;

    /// Get the diagonal border color index.
    fn get_diagonal_border_color(&self) -> u16;

    /// Get the diagonal border color object.
    fn get_diagonal_border_color_color(&self) -> Option<&dyn Color>;

    /// Get the left border color index.
    fn get_left_border_color(&self) -> u16;

    /// Get the left border color object.
    fn get_left_border_color_color(&self) -> Option<&dyn Color>;

    /// Get the right border color index.
    fn get_right_border_color(&self) -> u16;

    /// Get the right border color object.
    fn get_right_border_color_color(&self) -> Option<&dyn Color>;

    /// Get the top border color index.
    fn get_top_border_color(&self) -> u16;

    /// Get the top border color object.
    fn get_top_border_color_color(&self) -> Option<&dyn Color>;

    /// Get the vertical border color index.
    ///
    /// Range internal borders. Only relevant for range styles, such as table formatting.
    ///

    fn get_vertical_border_color(&self) -> u16;

    /// Get the vertical border color object.
    ///
    /// Range internal borders. Only relevant for range styles, such as table formatting.
    ///

    fn get_vertical_border_color_color(&self) -> Option<&dyn Color>;

    /// Get the horizontal border color index.
    ///
    /// Range internal borders. Only relevant for range styles, such as table formatting.
    ///

    fn get_horizontal_border_color(&self) -> u16;

    /// Get the horizontal border color object.
    ///
    /// Range internal borders. Only relevant for range styles, such as table formatting.
    ///

    fn get_horizontal_border_color_color(&self) -> Option<&dyn Color>;

    /// Set bottom border.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    fn set_border_bottom(&mut self, border: BorderStyle);

    /// Set diagonal border.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    fn set_border_diagonal(&mut self, border: BorderStyle);

    /// Set left border.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    fn set_border_left(&mut self, border: BorderStyle);

    /// Set right border.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    fn set_border_right(&mut self, border: BorderStyle);

    /// Set top border.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    fn set_border_top(&mut self, border: BorderStyle);

    /// Set range internal horizontal borders.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    ///

    fn set_border_horizontal(&mut self, border: BorderStyle);

    /// Set range internal vertical borders.
    ///
    /// # Arguments
    /// * `border` - The style of border to set.
    ///

    fn set_border_vertical(&mut self, border: BorderStyle);

    /// Set bottom border color by index.
    ///
    /// # Arguments
    /// * `color` - Color index.
    fn set_bottom_border_color_index(&mut self, color: u16);

    /// Set bottom border color by object.
    ///
    /// # Arguments
    /// * `color` - Color object.
    fn set_bottom_border_color(&mut self, color: Box<dyn Color>);

    /// Set diagonal border color by index.
    ///
    /// # Arguments
    /// * `color` - Color index.
    fn set_diagonal_border_color_index(&mut self, color: u16);

    /// Set diagonal border color by object.
    ///
    /// # Arguments
    /// * `color` - Color object.
    fn set_diagonal_border_color(&mut self, color: Box<dyn Color>);

    /// Set left border color by index.
    ///
    /// # Arguments
    /// * `color` - Color index.
    fn set_left_border_color_index(&mut self, color: u16);

    /// Set left border color by object.
    ///
    /// # Arguments
    /// * `color` - Color object.
    fn set_left_border_color(&mut self, color: Box<dyn Color>);

    /// Set right border color by index.
    ///
    /// # Arguments
    /// * `color` - Color index.
    fn set_right_border_color_index(&mut self, color: u16);

    /// Set right border color by object.
    ///
    /// # Arguments
    /// * `color` - Color object.
    fn set_right_border_color(&mut self, color: Box<dyn Color>);

    /// Set top border color by index.
    ///
    /// # Arguments
    /// * `color` - Color index.
    fn set_top_border_color_index(&mut self, color: u16);

    /// Set top border color by object.
    ///
    /// # Arguments
    /// * `color` - Color object.
    fn set_top_border_color(&mut self, color: Box<dyn Color>);

    /// Set horizontal border color by index.
    ///
    /// Range internal border color, such as table styles.
    ///
    /// # Arguments
    /// * `color` - Color index.
    ///

    fn set_horizontal_border_color_index(&mut self, color: u16);

    /// Set horizontal border color by object.
    ///
    /// Range internal border color, such as table styles.
    ///
    /// # Arguments
    /// * `color` - Color object.
    ///

    fn set_horizontal_border_color(&mut self, color: Box<dyn Color>);

    /// Set vertical border color by index.
    ///
    /// Range internal border color, such as table styles.
    ///
    /// # Arguments
    /// * `color` - Color index.
    ///

    fn set_vertical_border_color_index(&mut self, color: u16);

    /// Set vertical border color by object.
    ///
    /// Range internal border color, such as table styles.
    ///
    /// # Arguments
    /// * `color` - Color object.
    ///

    fn set_vertical_border_color(&mut self, color: Box<dyn Color>);
}
