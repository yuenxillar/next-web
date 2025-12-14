use std::fmt::Debug;

use crate::core::poi::ss::usermodel::{
    color::Color, conditional_formatting_threshold::ConditionalFormattingThreshold,
};

/// High level representation for the DataBar Formatting
/// component of Conditional Formatting settings
pub trait DataBarFormatting: Debug {
    /// Check if the bar is drawn from Left-to-Right, or from Right-to-Left.
    ///
    /// # Returns
    /// `true` if drawn from Left-to-Right, `false` if from Right-to-Left.
    fn is_left_to_right(&self) -> bool;

    /// Control if the bar is drawn from Left-to-Right, or from Right-to-Left.
    ///
    /// # Arguments
    /// * `ltr` - `true` for Left-to-Right, `false` for Right-to-Left.
    fn set_left_to_right(&mut self, ltr: bool);

    /// Check if only the icon should be displayed, or icon + value.
    ///
    /// # Returns
    /// `true` if only the icon is shown, `false` if icon + value are shown.
    fn is_icon_only(&self) -> bool;

    /// Control if only the icon is shown, or icon + value.
    ///
    /// # Arguments
    /// * `only` - `true` for icon only, `false` for icon + value.
    fn set_icon_only(&mut self, only: bool);

    /// Get how much of the cell width, in percentage, should be given to the min value.
    ///
    /// # Returns
    /// Width percentage for minimum value (0-100).
    fn get_width_min(&self) -> u8;

    /// Set how much of the cell width, in percentage, should be given to the min value.
    ///
    /// # Arguments
    /// * `width` - Width percentage for minimum value (0-100).
    fn set_width_min(&mut self, width: u8);

    /// Get how much of the cell width, in percentage, should be given to the max value.
    ///
    /// # Returns
    /// Width percentage for maximum value (0-100).
    fn get_width_max(&self) -> u8;

    /// Set how much of the cell width, in percentage, should be given to the max value.
    ///
    /// # Arguments
    /// * `width` - Width percentage for maximum value (0-100).
    fn set_width_max(&mut self, width: u8);

    /// Get the color of the data bar.
    ///
    /// # Returns
    /// The color of the data bar, or `None` if not set.
    fn get_color(&self) -> Option<&dyn Color>;

    /// Set the color of the data bar.
    ///
    /// # Arguments
    /// * `color` - The color to use for the data bar.
    fn set_color(&mut self, color: Box<dyn Color>);

    /// Get the threshold that defines "everything from here down is minimum".
    ///
    /// # Returns
    /// The minimum threshold.
    fn get_min_threshold(&self) -> Option<&dyn ConditionalFormattingThreshold>;

    /// Get the threshold that defines "everything from here up is maximum".
    ///
    /// # Returns
    /// The maximum threshold.
    fn get_max_threshold(&self) -> Option<&dyn ConditionalFormattingThreshold>;
}

/// Extended data bar formatting with validation and utility methods.
pub trait ExtendedDataBarFormatting: DataBarFormatting {
    /// Check if the data bar formatting is valid.
    ///
    /// # Returns
    /// `true` if valid, `false` otherwise.
    fn is_valid(&self) -> bool;

    /// Validate the width settings.
    ///
    /// # Returns
    /// `true` if width_min <= width_max, `false` otherwise.
    fn validate_widths(&self) -> bool {
        self.get_width_min() <= self.get_width_max()
    }

    /// Get the total width percentage allocated to the data bar.
    ///
    /// # Returns
    /// Total width percentage (width_min + width_max).
    fn get_total_width(&self) -> u8 {
        self.get_width_min() + self.get_width_max()
    }

    /// Check if the data bar uses the full cell width.
    ///
    /// # Returns
    /// `true` if total width is 100%, `false` otherwise.
    fn uses_full_width(&self) -> bool {
        self.get_total_width() == 100
    }

    /// Reset to default values.
    fn reset_to_defaults(&mut self);
}
