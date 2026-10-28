use std::fmt::Debug;

use crate::core::poi::common::usermodel::hyperlink_type::HyperlinkType;

/// Represents a hyperlink.
pub trait Hyperlink: Debug {
    /// Hyperlink address. Depending on the hyperlink type it can be URL, e-mail, path to a file, etc.
    ///
    /// # Returns
    /// The address of this hyperlink.
    fn get_address(&self) -> &str;

    /// Hyperlink address. Depending on the hyperlink type it can be URL, e-mail, path to a file, etc.
    ///
    /// # Arguments
    /// * `address` - The address of this hyperlink.
    fn set_address(&mut self, address: String);

    /// Return text label for this hyperlink.
    ///
    /// # Returns
    /// Text to display.
    fn get_label(&self) -> &str;

    /// Sets text label for this hyperlink.
    ///
    /// # Arguments
    /// * `label` - Text label for this hyperlink.
    fn set_label(&mut self, label: String);

    /// Return the type of this hyperlink.
    ///
    /// # Returns
    /// The type of this hyperlink.
    fn get_type(&self) -> HyperlinkType;
}
