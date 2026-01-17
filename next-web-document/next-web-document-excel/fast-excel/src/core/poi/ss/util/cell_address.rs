use std::cmp::Ordering;

use crate::core::poi::ss::{usermodel::cell::Cell, util::cell_reference::CellReference};

/// Container for POI usermodel row=0 column=0 cell references.
///
/// This is primarily a container for row and column coordinates.
/// The implementation of the `Comparable`/`Ord` interface sorts by "natural" order
/// from top left to bottom right.
///
/// Use `CellAddress` when you want to refer to the location of a cell in a sheet
/// when the concept of relative/absolute does not apply (such as the anchor location
/// of a cell comment). Use `CellReference` when the concept of relative/absolute
/// does apply (such as a cell reference in a formula). `CellAddress` does not have
/// a concept of "sheet", while `CellReference` does.
#[derive(Debug, Clone)]
pub struct CellAddress {
    /// Row index (0-based)
    row: i32,
    /// Column index (0-based)
    col: i32,
}

impl CellAddress {
    /// A constant for references to the first cell in a sheet (A1).
    pub const A1: CellAddress = CellAddress { row: 0, col: 0 };

    /// Creates a new CellAddress object.
    ///
    /// # Arguments
    /// * `row` - Row index (first row is 0)
    /// * `column` - Column index (first column is 0)
    pub fn new(row: i32, col: i32) -> Self {
        Self { row, col }
    }

    /// Creates a new CellAddress object from an A1 format string.
    ///
    /// # Arguments
    /// * `address` - A cell address in A1 format. Address must not contain sheet name or dollar signs.
    ///               (That is, address is not a cell reference. Use `CellAddress::from_cell_reference`
    ///               instead if starting with a cell reference.)
    ///
    /// # Panics
    /// Panics if the address format is invalid or contains non-digit characters after the column letters.
    pub fn from_a1_string(address: &str) -> Self {
        let mut loc = 0;

        // Step over column name chars until first digit for row number
        for (i, ch) in address.chars().enumerate() {
            if ch.is_ascii_digit() {
                loc = i;
                break;
            }
        }

        let s_col = address[..loc].to_uppercase();
        let s_row = &address[loc..];

        // FIXME: breaks if address contains a sheet name or dollar signs from an absolute CellReference
        let row = s_row.parse::<i32>().expect("Invalid row number in address") - 1;
        let col = CellReference::convert_col_string_to_index(&s_col);

        Self { row, col }
    }

    /// Creates a new CellAddress object from a CellReference.
    ///
    /// # Arguments
    /// * `reference` - A reference to a cell
    pub fn from_cell_reference(reference: &CellReference) -> Self {
        Self::new(reference.get_row(), reference.get_col())
    }

    /// Creates a new CellAddress object from another CellAddress.
    ///
    /// # Arguments
    /// * `address` - A CellAddress to copy
    pub fn from_cell_address(address: &CellAddress) -> Self {
        Self::new(address.get_row(), address.get_column())
    }

    /// Creates a new CellAddress object from a Cell.
    ///
    /// # Arguments
    /// * `cell` - The Cell to get the location of
    pub fn from_cell(cell: &dyn Cell) -> Self {
        Self::new(
            cell.get_row_index().map(|n| n as i32).unwrap_or(-1),
            cell.get_column_index().map(|n| n as i32).unwrap_or(-1),
        )
    }

    /// Gets the cell address row.
    ///
    /// # Returns
    /// Row index (0-based)
    pub fn get_row(&self) -> i32 {
        self.row
    }

    /// Gets the cell address column.
    ///
    /// # Returns
    /// Column index (0-based)
    pub fn get_column(&self) -> i32 {
        self.col
    }

    /// Formats the cell address as an A1-style string.
    ///
    /// # Returns
    /// A1-style cell address string representation
    pub fn format_as_string(&self) -> String {
        format!(
            "{}{}",
            CellReference::convert_num_to_col_string(self.col),
            self.row + 1
        )
    }

    /// Formats the cell address as an R1C1-style string.
    ///
    /// # Returns
    /// R1C1-style cell address string representation
    pub fn format_as_r1c1_string(&self) -> String {
        CellReference::from_indices(self.row, self.col).format_as_r1c1_string()
    }
}

impl PartialEq for CellAddress {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row && self.col == other.col
    }
}

impl Eq for CellAddress {}

impl Ord for CellAddress {
    /// Compares this CellAddress using the "natural" row-major, column-minor ordering.
    /// That is, top-left to bottom-right ordering.
    ///
    /// # Returns
    /// * `Ordering::Less` if this CellAddress is before (above/left) of other
    /// * `Ordering::Equal` if addresses are the same
    /// * `Ordering::Greater` if this CellAddress is after (below/right) of other
    fn cmp(&self, other: &Self) -> Ordering {
        match self.row.cmp(&other.row) {
            Ordering::Equal => self.col.cmp(&other.col),
            ordering => ordering,
        }
    }
}

impl PartialOrd for CellAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for CellAddress {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.row + self.col) << 16).hash(state);
    }
}

impl std::fmt::Display for CellAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_as_string())
    }
}
