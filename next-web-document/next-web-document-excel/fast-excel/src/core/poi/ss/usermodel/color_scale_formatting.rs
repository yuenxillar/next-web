use crate::core::poi::ss::usermodel::{
    color::Color, conditional_formatting_threshold::ConditionalFormattingThreshold,
};

/// High level representation for the Color Scale / Colour Scale /
/// Color Gradient Formatting component of Conditional Formatting settings
pub trait ColorScaleFormatting {
    /// Get how many control points should be used to map the colours
    /// Normally 2 or 3
    fn get_num_control_points(&self) -> i32;

    /// Set the number of control points to use to map the colours
    /// Should normally be 2 or 3.
    /// After updating, you need to ensure that the Threshold count and Color count match
    fn set_num_control_points(&mut self, num: i32);

    /// Get the list of colours that are interpolated between
    fn get_colors(&self) -> Vec<&dyn Color>;

    /// Set the list of colours that are interpolated between
    /// The number must match `get_num_control_points()`
    fn set_colors(&mut self, colors: Vec<&dyn Color>);

    /// Get the list of thresholds
    fn get_thresholds(&self) -> Vec<Box<dyn ConditionalFormattingThreshold>>;

    /// Set the list of thresholds
    /// The number must match `get_num_control_points()`
    fn set_thresholds(&mut self, thresholds: Vec<Box<dyn ConditionalFormattingThreshold>>);

    /// Create a new, empty Threshold
    fn create_threshold(&self) -> Box<dyn ConditionalFormattingThreshold>;
}
