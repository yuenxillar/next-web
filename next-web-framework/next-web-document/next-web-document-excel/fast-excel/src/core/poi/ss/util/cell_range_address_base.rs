use std::collections::HashSet;
use std::fmt;

/// See OOO documentation: excelfileformat.pdf sec 2.5.14 - 'Cell Range Address'
///
/// Common superclass of 8-bit and 16-bit versions
pub struct CellRangeAddressBase {
    /// First row index (0-based)
    first_row: u32,
    /// First column index (0-based)
    first_col: u32,
    /// Last row index (0-based, inclusive)
    last_row: u32,
    /// Last column index (0-based, inclusive)
    last_col: u32,
}

impl CellRangeAddressBase {
    /// Creates a new cell range address base
    pub fn new(first_row: u32, last_row: u32, first_col: u32, last_col: u32) -> Self {
        CellRangeAddressBase {
            first_row,
            last_row,
            first_col,
            last_col,
        }
    }

    /// Get the first column index
    pub fn get_first_column(&self) -> i32 {
        self.first_col
    }

    /// Get the first row index
    pub fn get_first_row(&self) -> i32 {
        self.first_row
    }

    /// Get the last column index
    pub fn get_last_column(&self) -> i32 {
        self.last_col
    }

    /// Get the last row index
    pub fn get_last_row(&self) -> i32 {
        self.last_row
    }

    /// Set the first column index
    pub fn set_first_column(&mut self, first_col: i32) {
        self.first_col = first_col;
    }

    /// Set the first row index
    pub fn set_first_row(&mut self, first_row: i32) {
        self.first_row = first_row;
    }

    /// Set the last column index
    pub fn set_last_column(&mut self, last_col: i32) {
        self.last_col = last_col;
    }

    /// Set the last row index
    pub fn set_last_row(&mut self, last_row: i32) {
        self.last_row = last_row;
    }

    /// Check if the range is a full column range
    pub fn is_full_column_range(&self) -> bool {
        // Excel 97 maximum rows
        const EXCEL97_MAX_ROW: i32 = 65535;
        (self.first_row == 0 && self.last_row == EXCEL97_MAX_ROW)
            || (self.first_row == -1 && self.last_row == -1)
    }

    /// Check if the range is a full row range
    pub fn is_full_row_range(&self) -> bool {
        // Excel 97 maximum columns
        const EXCEL97_MAX_COL: i32 = 255;
        (self.first_col == 0 && self.last_col == EXCEL97_MAX_COL)
            || (self.first_col == -1 && self.last_col == -1)
    }

    /// Check if coordinates are within the range
    pub fn is_in_range(&self, row_ind: i32, col_ind: i32) -> bool {
        self.first_row <= row_ind
            && row_ind <= self.last_row
            && self.first_col <= col_ind
            && col_ind <= self.last_col
    }

    /// Check if the row is in the range
    pub fn contains_row(&self, row_ind: i32) -> bool {
        self.first_row <= row_ind && row_ind <= self.last_row
    }

    /// Check if the column is in the range
    pub fn contains_column(&self, col_ind: i32) -> bool {
        self.first_col <= col_ind && col_ind <= self.last_col
    }

    /// Check if this range intersects with another range
    pub fn intersects(&self, other: &CellRangeAddressBase) -> bool {
        self.first_row <= other.last_row
            && self.first_col <= other.last_col
            && other.first_row <= self.last_row
            && other.first_col <= self.last_col
    }

    /// Get the position of a cell within this range
    pub fn get_position(&self, row_ind: i32, col_ind: i32) -> HashSet<CellPosition> {
        let mut positions = HashSet::new();

        if row_ind > self.first_row
            && row_ind < self.last_row
            && col_ind > self.first_col
            && col_ind < self.last_col
        {
            positions.insert(CellPosition::Inside);
            return positions;
        }

        if row_ind == self.first_row {
            positions.insert(CellPosition::Top);
        }
        if row_ind == self.last_row {
            positions.insert(CellPosition::Bottom);
        }
        if col_ind == self.first_col {
            positions.insert(CellPosition::Left);
        }
        if col_ind == self.last_col {
            positions.insert(CellPosition::Right);
        }

        positions
    }

    /// Get the number of cells in the range
    pub fn get_number_of_cells(&self) -> i32 {
        (self.last_row - self.first_row + 1) * (self.last_col - self.first_col + 1)
    }

    /// Get the minimum row (handles reversed ranges)
    pub fn get_min_row(&self) -> i32 {
        self.first_row.min(self.last_row)
    }

    /// Get the maximum row (handles reversed ranges)
    pub fn get_max_row(&self) -> i32 {
        self.first_row.max(self.last_row)
    }

    /// Get the minimum column (handles reversed ranges)
    pub fn get_min_column(&self) -> i32 {
        self.first_col.min(self.last_col)
    }

    /// Get the maximum column (handles reversed ranges)
    pub fn get_max_column(&self) -> i32 {
        self.first_col.max(self.last_col)
    }
}

impl fmt::Display for CellRangeAddressBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CellRangeAddressBase [{},{}:{},{}]",
            self.first_row, self.first_col, self.last_row, self.last_col
        )
    }
}

impl PartialEq for CellRangeAddressBase {
    fn eq(&self, other: &Self) -> bool {
        self.get_min_row() == other.get_min_row()
            && self.get_max_row() == other.get_max_row()
            && self.get_min_column() == other.get_min_column()
            && self.get_max_column() == other.get_max_column()
    }
}

impl Eq for CellRangeAddressBase {}

impl std::hash::Hash for CellRangeAddressBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get_min_column().hash(state);
        self.get_max_column().hash(state);
        self.get_min_row().hash(state);
        self.get_max_row().hash(state);
    }
}

/// Indicates a cell or range is in the given relative position in a range.
/// More than one of these may apply at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellPosition {
    /// Range starting rows are equal
    Top,
    /// Range ending rows are equal
    Bottom,
    /// Range starting columns are equal
    Left,
    /// Range ending columns are equal
    Right,
    /// A cell or range is completely inside another range,
    /// without touching any edges (a cell in this position can't be in any others)
    Inside,
}

/// Iterator over cell addresses in row-major order
pub struct RowMajorCellAddressIterator {
    first_row: i32,
    first_col: i32,
    last_row: i32,
    last_col: i32,
    current_row: i32,
    current_col: i32,
}

impl RowMajorCellAddressIterator {
    pub fn new(range: &CellRangeAddressBase) -> Self {
        let first_row = range.get_first_row();
        let first_col = range.get_first_column();
        let last_row = range.get_last_row();
        let last_col = range.get_last_column();

        // Validate the range
        if first_row < 0 {
            panic!("First row cannot be negative.");
        }
        if first_col < 0 {
            panic!("First column cannot be negative.");
        }
        if first_row > last_row {
            panic!("First row cannot be greater than last row.");
        }
        if first_col > last_col {
            panic!("First column cannot be greater than last column.");
        }

        RowMajorCellAddressIterator {
            first_row,
            first_col,
            last_row,
            last_col,
            current_row: first_row,
            current_col: first_col,
        }
    }
}

impl Iterator for RowMajorCellAddressIterator {
    type Item = CellAddress;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row > self.last_row || self.current_col > self.last_col {
            return None;
        }

        let addr = CellAddress::new(self.current_row, self.current_col);

        // Row-major order: move to next column, then next row
        if self.current_col < self.last_col {
            self.current_col += 1;
        } else {
            self.current_col = self.first_col;
            self.current_row += 1;
        }

        Some(addr)
    }
}

/// Simple cell address structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellAddress {
    row: i32,
    column: i32,
}

impl CellAddress {
    pub fn new(row: i32, column: i32) -> Self {
        CellAddress { row, column }
    }

    pub fn get_row(&self) -> i32 {
        self.row
    }

    pub fn get_column(&self) -> i32 {
        self.column
    }
}
