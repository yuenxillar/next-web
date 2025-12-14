use crate::core::poi::ss::usermodel::child_anchor::ChildAnchor;

/// Common interface for all drawing shapes
pub trait Shape: Send + Sync {
    /// Get the name of this shape
    ///
    /// # Returns
    /// * Name of the shape
    fn get_shape_name(&self) -> &str;

    /// Get the parent shape
    ///
    /// # Returns
    /// * Parent shape or `None` if this is a top-level shape
    fn get_parent(&self) -> Option<Box<dyn Shape>>;

    /// Get the anchor that is used by this shape
    ///
    /// # Returns
    /// * Child anchor for positioning
    fn get_anchor(&self) -> Option<&dyn ChildAnchor>;

    /// Check if this shape is not filled with a color
    ///
    /// # Returns
    /// * `true` if this shape is not filled with a color
    fn is_no_fill(&self) -> bool;

    /// Set whether this shape is filled or transparent
    ///
    /// # Arguments
    /// * `no_fill` - If `true` then no fill will be applied to the shape element
    fn set_no_fill(&mut self, no_fill: bool);

    /// Set the color used to fill this shape using the solid fill pattern
    ///
    /// # Arguments
    /// * `red` - Red component (0-255)
    /// * `green` - Green component (0-255)
    /// * `blue` - Blue component (0-255)
    fn set_fill_color(&mut self, red: u8, green: u8, blue: u8);

    /// Set the color applied to the lines of this shape
    ///
    /// # Arguments
    /// * `red` - Red component (0-255)
    /// * `green` - Green component (0-255)
    /// * `blue` - Blue component (0-255)
    fn set_line_style_color(&mut self, red: u8, green: u8, blue: u8);
}
