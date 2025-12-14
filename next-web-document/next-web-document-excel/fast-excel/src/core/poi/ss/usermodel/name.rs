/// Represents a defined name for a range of cells.
///
/// A name is a meaningful shorthand that makes it easier to understand the purpose of a
/// cell reference, constant or a formula.
pub trait Name: Send + Sync {
    /// Get the sheet name which this named range is referenced to
    ///
    /// # Returns
    /// * Sheet name which this named range referred to
    fn get_sheet_name(&self) -> Option<&str>;

    /// Gets the name of the named range
    ///
    /// # Returns
    /// * Named range name
    fn get_name_name(&self) -> &str;

    /// Sets the name of the named range
    ///
    /// The following is a list of syntax rules that you need to be aware of when you create and edit names.
    ///
    /// **Valid characters**
    /// The first character of a name must be a letter, an underscore character (_), or a backslash (\).
    /// Remaining characters in the name can be letters, numbers, periods, and underscore characters.
    ///
    /// **Cell references disallowed**
    /// Names cannot be the same as a cell reference, such as Z$100 or R1C1.
    ///
    /// **Spaces are not valid**
    /// Spaces are not allowed as part of a name. Use the underscore character (_) and period (.) as word separators, such as, Sales_Tax or First.Quarter.
    ///
    /// **Name length**
    /// A name can contain up to 255 characters.
    ///
    /// **Case sensitivity**
    /// Names can contain uppercase and lowercase letters.
    ///
    /// A name must always be unique within its scope. POI prevents you from defining a name that is not unique
    /// within its scope. However you can use the same name in different scopes.
    ///
    /// # Arguments
    /// * `name` - Named range name to set
    ///
    /// # Errors
    /// * Returns error if the name is invalid or already exists within its scope (case-insensitive)
    fn set_name_name(&mut self, name: &str) -> Result<(), String>;

    /// Returns the formula that the name is defined to refer to.
    ///
    /// # Returns
    /// * The reference for this name, `None` if it has not been set yet
    fn get_refers_to_formula(&self) -> Option<&str>;

    /// Sets the formula that the name is defined to refer to. Examples:
    ///
    /// - `'My Sheet'!$A$3`
    /// - `8.3`
    /// - `HR!$A$1:$Z$345`
    /// - `SUM(Sheet1!A1,Sheet2!B2)`
    /// - `-PMT(Interest_Rate/12,Number_of_Payments,Loan_Amount)`
    ///
    /// Note: Using relative values like 'A1:B1' can lead to unexpected moving of
    /// the cell that the name points to when working with the workbook in Microsoft Excel,
    /// usually using absolute references like '$A$1:$B$1' avoids this.
    ///
    /// # Arguments
    /// * `formula_text` - The reference for this name
    ///
    /// # Errors
    /// * Returns error if the specified formula_text is unparsable
    fn set_refers_to_formula(&mut self, formula_text: &str) -> Result<(), String>;

    /// Checks if this name is a function name
    ///
    /// # Returns
    /// * `true` if this name is a function name
    fn is_function_name(&self) -> bool;

    /// Checks if this name points to a cell that no longer exists
    ///
    /// # Returns
    /// * `true` if the name refers to a deleted cell, `false` otherwise
    fn is_deleted(&self) -> bool;

    /// Checks if this name is hidden, eg one of the built-in Excel internal names
    ///
    /// # Returns
    /// * `true` if the name is a hidden name, `false` otherwise
    fn is_hidden(&self) -> bool;

    /// Tell Excel that this name applies to the worksheet with the specified index instead of the entire workbook.
    ///
    /// # Arguments
    /// * `sheet_id` - The sheet index this name applies to, `None` unsets this property making the name workbook-global
    ///
    /// # Errors
    /// * Returns error if the sheet index is invalid
    fn set_sheet_index(&mut self, sheet_id: Option<u32>) -> Result<(), String>;

    /// Returns the sheet index this name applies to.
    ///
    /// # Returns
    /// * The sheet index this name applies to, `None` if this name applies to the entire workbook
    fn get_sheet_index(&self) -> Option<usize>;

    /// Returns the comment the user provided when the name was created.
    ///
    /// # Returns
    /// * The user comment for this named range
    fn get_comment(&self) -> Option<&str>;

    /// Sets the comment the user provided when the name was created.
    ///
    /// # Arguments
    /// * `comment` - The user comment for this named range
    fn set_comment(&mut self, comment: &str);

    /// Indicates that the defined name refers to a user-defined function.
    /// This attribute is used when there is an add-in or other code project associated with the file.
    ///
    /// # Arguments
    /// * `value` - `true` indicates the name refers to a function
    fn set_function(&mut self, value: bool);
}
