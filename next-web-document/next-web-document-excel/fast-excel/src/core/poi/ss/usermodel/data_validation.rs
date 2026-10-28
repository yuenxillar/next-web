use crate::core::poi::ss::{
    usermodel::data_validation_constraint::DataValidationConstraint,
    util::cell_range_address_list::CellRangeAddressList,
};

/// Represents data validation settings for cells in a worksheet.
pub trait DataValidation {
    /// Gets the validation constraint for this data validation.
    ///
    /// # Returns
    /// The validation constraint.
    fn get_validation_constraint(&self) -> Box<dyn DataValidationConstraint>;

    /// Sets the error style for error box.
    ///
    /// # Arguments
    /// * `error_style` - The error style to set.
    fn set_error_style(&mut self, error_style: i32);

    /// Gets the error style of error box.
    ///
    /// # Returns
    /// The error style.
    fn get_error_style(&self) -> i32;

    /// Sets if this object allows empty as a valid value.
    ///
    /// # Arguments
    /// * `allowed` - `true` if this object should treat empty as valid value, `false` otherwise.
    fn set_empty_cell_allowed(&mut self, allowed: bool);

    /// Retrieve the settings for empty cells allowed.
    ///
    /// # Returns
    /// `true` if this object should treat empty as valid value, `false` otherwise.
    fn get_empty_cell_allowed(&self) -> bool;

    /// Useful for list validation objects.
    ///
    /// # Arguments
    /// * `suppress` - `true` if a list should suppress the drop-down arrow, `false` otherwise.
    ///                In other words, if a list should display the arrow sign on its right side.
    fn set_suppress_drop_down_arrow(&mut self, suppress: bool);

    /// Useful only for list validation objects.
    /// This method always returns `false` if the object isn't a list validation object.
    ///
    /// # Returns
    /// `true` if a list should suppress the drop-down arrow, `false` otherwise.
    fn get_suppress_drop_down_arrow(&self) -> bool;

    /// Sets the behaviour when a cell which belongs to this object is selected.
    ///
    /// # Arguments
    /// * `show` - `true` if a prompt box should be displayed, `false` otherwise.
    fn set_show_prompt_box(&mut self, show: bool);

    /// Gets whether a prompt box should be displayed.
    ///
    /// # Returns
    /// `true` if a prompt box should be displayed, `false` otherwise.
    fn get_show_prompt_box(&self) -> bool;

    /// Sets the behaviour when an invalid value is entered.
    ///
    /// # Arguments
    /// * `show` - `true` if an error box should be displayed, `false` otherwise.
    fn set_show_error_box(&mut self, show: bool);

    /// Gets whether an error box should be displayed.
    ///
    /// # Returns
    /// `true` if an error box should be displayed, `false` otherwise.
    fn get_show_error_box(&self) -> bool;

    /// Sets the title and text for the prompt box.
    /// Prompt box is displayed when the user selects a cell which belongs to this validation object.
    /// In order for a prompt box to be displayed you should also use method `set_show_prompt_box(true)`.
    ///
    /// # Arguments
    /// * `title` - The prompt box's title.
    /// * `text` - The prompt box's text.
    fn create_prompt_box(&mut self, title: String, text: String);

    /// Gets the prompt box's title.
    ///
    /// # Returns
    /// Prompt box's title or `None`.
    fn get_prompt_box_title(&self) -> Option<&str>;

    /// Gets the prompt box's text.
    ///
    /// # Returns
    /// Prompt box's text or `None`.
    fn get_prompt_box_text(&self) -> Option<&str>;

    /// Sets the title and text for the error box.
    /// Error box is displayed when the user enters an invalid value into a cell which belongs to this validation object.
    /// In order for an error box to be displayed you should also use method `set_show_error_box(true)`.
    ///
    /// # Arguments
    /// * `title` - The error box's title.
    /// * `text` - The error box's text.
    fn create_error_box(&mut self, title: String, text: String);

    /// Gets the error box's title.
    ///
    /// # Returns
    /// Error box's title or `None`.
    fn get_error_box_title(&self) -> Option<&str>;

    /// Gets the error box's text.
    ///
    /// # Returns
    /// Error box's text or `None`.
    fn get_error_box_text(&self) -> Option<&str>;

    /// Gets the cell regions that this data validation applies to.
    ///
    /// # Returns
    /// The cell range address list.
    fn get_regions(&self) -> CellRangeAddressList;
}

/// Error style constants for error box
pub mod error_style {
    /// STOP style
    pub const STOP: i32 = 0x00;
    /// WARNING style
    pub const WARNING: i32 = 0x01;
    /// INFO style
    pub const INFO: i32 = 0x02;
}
