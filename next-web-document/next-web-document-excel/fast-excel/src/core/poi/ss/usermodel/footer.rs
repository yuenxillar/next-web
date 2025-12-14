use crate::core::poi::ss::usermodel::header_footer::HeaderFooter;

/// Common definition of a HSSF or XSSF page footer.
/// For a list of all the different fields that can be
/// placed into a footer, such as page number,
/// bold, underline etc, see `HeaderFooter`.
pub trait Footer: HeaderFooter {
    /// Get the left side of the footer.
    ///
    /// # Returns
    /// The string representing the left side.
    fn get_left(&self) -> &str;

    /// Sets the left string.
    ///
    /// # Arguments
    /// * `new_left` - The string to set as the left side.
    fn set_left(&mut self, new_left: String);

    /// Get the center of the footer.
    ///
    /// # Returns
    /// The string representing the center.
    fn get_center(&self) -> &str;

    /// Sets the center string.
    ///
    /// # Arguments
    /// * `new_center` - The string to set as the center.
    fn set_center(&mut self, new_center: String);

    /// Get the right side of the footer.
    ///
    /// # Returns
    /// The string representing the right side.
    fn get_right(&self) -> &str;

    /// Sets the right string.
    ///
    /// # Arguments
    /// * `new_right` - The string to set as the right side.
    fn set_right(&mut self, new_right: String);
}
