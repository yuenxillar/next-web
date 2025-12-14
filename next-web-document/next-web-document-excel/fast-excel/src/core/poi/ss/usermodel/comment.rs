use std::fmt::Debug;

use crate::core::poi::ss::{
    usermodel::{client_anchor::ClientAnchor, rich_text_string::RichTextString},
    util::cell_range_address_base::CellAddress,
};

/// Represents a cell comment in a spreadsheet.
/// Comments are notes attached to cells that can be displayed when hovering over the cell.
pub trait Comment: Debug {
    /// Sets whether this comment is visible.
    ///
    /// # Arguments
    /// * `visible` - `true` if the comment should be visible, `false` otherwise
    fn set_visible(&mut self, visible: bool);

    /// Returns whether this comment is visible.
    ///
    /// # Returns
    /// `true` if the comment is visible, `false` otherwise
    fn is_visible(&self) -> bool;

    /// Get the address of the cell that this comment is attached to.
    ///
    /// # Returns
    /// The cell address where this comment is located
    fn get_address(&self) -> Option<&CellAddress>;

    /// Set the address of the cell that this comment is attached to using a `CellAddress`.
    ///
    /// # Arguments
    /// * `addr` - The cell address to attach this comment to
    fn set_address(&mut self, addr: CellAddress);

    /// Set the address of the cell that this comment is attached to using row and column indices.
    ///
    /// # Arguments
    /// * `row` - The 0-based row index
    /// * `col` - The 0-based column index
    fn set_address_by_indices(&mut self, row: u32, col: u32);

    /// Return the row of the cell that contains the comment.
    ///
    /// # Returns
    /// The 0-based row index of the cell that contains the comment
    fn get_row(&self) -> u32;

    /// Set the row of the cell that contains the comment.
    ///
    /// # Arguments
    /// * `row` - The 0-based row index of the cell that contains the comment
    fn set_row(&mut self, row: u32);

    /// Return the column of the cell that contains the comment.
    ///
    /// # Returns
    /// The 0-based column index of the cell that contains the comment
    fn get_column(&self) -> u32;

    /// Set the column of the cell that contains the comment.
    ///
    /// # Arguments
    /// * `col` - The 0-based column index of the cell that contains the comment
    fn set_column(&mut self, col: u32);

    /// Get the name of the original comment author.
    ///
    /// # Returns
    /// The name of the original author of the comment
    fn get_author(&self) -> &str;

    /// Set the name of the original comment author.
    ///
    /// # Arguments
    /// * `author` - The name of the original author of the comment
    fn set_author(&mut self, author: String);

    /// Fetches the rich text string of the comment.
    ///
    /// # Returns
    /// A reference to the rich text string content of the comment
    fn get_string(&self) -> &dyn RichTextString;

    /// Sets the rich text string used by this comment.
    ///
    /// # Arguments
    /// * `string` - The rich text string to set for this comment
    fn set_string(&mut self, string: Box<dyn RichTextString>);

    /// Return the defined position of this anchor in the sheet.
    /// The anchor is the yellow box/balloon that is rendered on top of the sheet
    /// when the comment is visible.
    ///
    /// To associate a comment with a different cell, use `set_address`.
    ///
    /// # Returns
    /// The defined position of this anchor in the sheet, wrapped in `Option` as it can be `None`
    fn get_client_anchor(&self) -> Option<&dyn ClientAnchor>;
}
