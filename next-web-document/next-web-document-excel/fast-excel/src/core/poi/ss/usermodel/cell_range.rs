use crate::core::poi::ss::usermodel::cell::Cell;

/// Represents a rectangular region of a `Sheet`.
pub trait CellRange<C: Cell>: Iterator<Item = C> {
    /// Gets the width (number of columns) of this cell range.
    ///
    /// # Returns
    /// The width of the range.
    fn get_width(&self) -> usize;

    /// Gets the height (number of rows) of this cell range.
    ///
    /// # Returns
    /// The height of the range.
    fn get_height(&self) -> usize;

    /// Gets the number of cells in this range.
    ///
    /// # Returns
    /// `height * width`
    fn size(&self) -> usize {
        self.get_width() * self.get_height()
    }

    /// Gets the text format of this range.
    ///
    /// # Returns
    /// The reference text. Single cell ranges are formatted
    /// like single cell references (e.g., 'A1' instead of 'A1:A1').
    fn get_reference_text(&self) -> Option<&str>;

    /// Gets the cell at relative coordinates (0, 0).
    ///
    /// # Returns
    /// The top-left cell. Never `None`.
    fn get_top_left_cell(&self) -> &C;

    /// Gets the cell at the specified relative coordinates.
    ///
    /// # Arguments
    /// * `relative_row_index` - must be between `0` and `height - 1`
    /// * `relative_column_index` - must be between `0` and `width - 1`
    ///
    /// # Returns
    /// The cell at the specified coordinates. Never `None`.
    ///
    /// # Panics
    /// Panics if the indices are out of bounds.
    fn get_cell(&self, relative_row_index: usize, relative_column_index: usize) -> &C;

    /// Gets a flattened vector of all the cells in this CellRange.
    ///
    /// # Returns
    /// A flattened vector of all cells.
    fn get_flattened_cells(&self) -> Vec<&C>;

    /// Gets a 2-D vector of all the cells in this CellRange.
    ///
    /// # Returns
    /// A 2-D vector where the first dimension is the row index
    /// (values `0...height - 1`) and the second dimension is the
    /// column index (values `0...width - 1`).
    fn get_cells(&self) -> Vec<Vec<&C>>;

    /// Creates an iterator over all cells in this range.
    ///
    /// # Returns
    /// An iterator that yields cells starting with all cells in the
    /// first row followed by all cells in the next row, etc.
    fn cell_iter(&self) -> Box<dyn Iterator<Item = &C> + '_>;
}
