use std::str::FromStr;
use std::{fmt, io};

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

use crate::core::poi::common::usermodel::generic_record::GenericRecord;
use crate::core::poi::ss::util::cell_reference::CellReference;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

/// See OOO documentation: excelfileformat.pdf sec 2.5.14 - 'Cell Range Address'
///
/// In the Microsoft documentation, this is also known as a Ref8U - see page 831 of version 1.0.
///
/// Note - `SelectionRecord` uses the BIFF5 version of this structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellRangeAddress {
    /// Index of first row (zero-based)
    first_row: u32,
    /// Index of last row (zero-based, inclusive)
    last_row: u32,
    /// Index of first column (zero-based)
    first_col: u32,
    /// Index of last column (zero-based, inclusive)
    last_col: u32,
}

impl CellRangeAddress {
    /// Encoded size in bytes for a single CellRangeAddress
    pub const ENCODED_SIZE: usize = 8;

    /// Creates new cell range. Indexes are zero-based.
    ///
    /// # Arguments
    /// * `first_row` - Index of first row
    /// * `last_row` - Index of last row (inclusive), must be equal to or larger than `first_row`
    /// * `first_col` - Index of first column
    /// * `last_col` - Index of last column (inclusive), must be equal to or larger than `first_col`
    ///
    /// # Panics
    /// Panics if `last_row < first_row || last_col < first_col`
    pub fn new(
        first_row: u32,
        last_row: u32,
        first_col: u32,
        last_col: u32,
    ) -> Result<Self, String> {
        if last_row < first_row || last_col < first_col {
            return Err(format!(
                "Invalid cell range, having lastRow < firstRow || lastCol < firstCol, \
                 had rows {} >= {} or cells {} >= {}",
                last_row, first_row, last_col, first_col
            ));
        }

        Ok(CellRangeAddress {
            first_row,
            last_row,
            first_col,
            last_col,
        })
    }

    /// Serializes the cell range address to a little-endian output
    pub fn serialize(&self, out: &mut dyn LittleEndianOutput) -> io::Result<()> {
        out.write_short(self.first_row as u16)?;
        out.write_short(self.last_row as u16)?;
        out.write_short(self.first_col as u16)?;
        out.write_short(self.last_col as u16)?;

        Ok(())
    }

    /// Deserializes a cell range address from a byte slice
    ///
    /// # Arguments
    /// * `data` - Byte slice containing the serialized data
    ///
    /// # Returns
    /// `Result<CellRangeAddress, String>` - The deserialized cell range address or an error
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < Self::ENCODED_SIZE {
            return Err("Ran out of data reading CellRangeAddress".to_string());
        }

        let first_row = u16::from_le_bytes([data[0], data[1]]) as i32;
        let last_row = u16::from_le_bytes([data[2], data[3]]) as i32;
        let first_col = u16::from_le_bytes([data[4], data[5]]) as i32;
        let last_col = u16::from_le_bytes([data[6], data[7]]) as i32;

        Ok(CellRangeAddress::new(
            first_row, last_row, first_col, last_col,
        ))
    }

    /// Creates a copy of this cell range address
    pub fn copy(&self) -> Self {
        *self
    }

    /// Gets the encoded size for a given number of items
    pub fn get_encoded_size(number_of_items: usize) -> usize {
        number_of_items * Self::ENCODED_SIZE
    }

    /// Returns the text format of this range.
    /// Single cell ranges are formatted like single cell references (e.g. 'A1' instead of 'A1:A1').
    pub fn format_as_string(&self) -> String {
        self.format_as_string_with_sheet(None, false)
    }

    /// Returns the text format of this range using specified sheet name.
    ///
    /// # Arguments
    /// * `sheet_name` - Optional sheet name
    /// * `use_absolute_address` - Whether to use absolute addressing (with $)
    pub fn format_as_string_with_sheet(
        &self,
        sheet_name: Option<&str>,
        use_absolute_address: bool,
    ) -> String {
        let mut sb = String::new();

        // Append sheet name if provided
        if let Some(name) = sheet_name {
            sb.push_str(&format_sheet_name(name));
            sb.push('!');
        }

        let cell_ref_from = CellReference::new(
            self.first_row,
            self.first_col,
            use_absolute_address,
            use_absolute_address,
        );
        let cell_ref_to = CellReference::new(
            self.last_row,
            self.last_col,
            use_absolute_address,
            use_absolute_address,
        );

        sb.push_str(&cell_ref_from.format_as_string());

        // For a single-cell reference return A1 instead of A1:A1
        // For full-column ranges or full-row ranges return A:A instead of A,
        // and 1:1 instead of 1
        if !cell_ref_from.eq(&cell_ref_to)
            || self.is_full_column_range()
            || self.is_full_row_range()
        {
            sb.push(':');
            sb.push_str(&cell_ref_to.format_as_string());
        }

        sb
    }

    /// Creates a CellRangeAddress from a cell range reference string.
    ///
    /// # Arguments
    /// * `ref_str` - Usually a standard area ref (e.g. "B1:D8").
    ///               May be a single cell ref (e.g. "B5") in which case the result is a 1 x 1 cell range.
    ///               May also be a whole row range (e.g. "3:5"), or a whole column range (e.g. "C:F")
    ///
    /// # Returns
    /// `Result<CellRangeAddress, String>` - The parsed cell range address or an error
    pub fn from_string(ref_str: &str) -> Result<Self, String> {
        let ref_str = ref_str.trim();

        if let Some(sep) = ref_str.find(':') {
            let a_str = &ref_str[..sep];
            let b_str = &ref_str[sep + 1..];

            let a = CellReference::from_string(a_str)
                .map_err(|e| format!("Invalid first cell reference '{}': {}", a_str, e))?;
            let b = CellReference::from_string(b_str)
                .map_err(|e| format!("Invalid second cell reference '{}': {}", b_str, e))?;

            Ok(CellRangeAddress::new(a.row, b.row, a.column, b.column))
        } else {
            let a = CellReference::from_string(ref_str)
                .map_err(|e| format!("Invalid cell reference '{}': {}", ref_str, e))?;
            Ok(CellRangeAddress::new(a.row, a.row, a.column, a.column))
        }
    }

    /// Gets the first row index
    pub fn get_first_row(&self) -> i32 {
        self.first_row
    }

    /// Gets the last row index
    pub fn get_last_row(&self) -> i32 {
        self.last_row
    }

    /// Gets the first column index
    pub fn get_first_column(&self) -> i32 {
        self.first_col
    }

    /// Gets the last column index
    pub fn get_last_column(&self) -> i32 {
        self.last_col
    }

    /// Returns true if this range spans the entire column
    pub fn is_full_column_range(&self) -> bool {
        // Assuming maximum rows in Excel
        self.first_row == 0 && self.last_row >= 1048575
    }

    /// Returns true if this range spans the entire row
    pub fn is_full_row_range(&self) -> bool {
        // Assuming maximum columns in Excel (XFD = 16383)
        self.first_col == 0 && self.last_col >= 16383
    }

    /// Returns the number of rows in this range
    pub fn get_row_count(&self) -> i32 {
        self.last_row - self.first_row + 1
    }

    /// Returns the number of columns in this range
    pub fn get_column_count(&self) -> i32 {
        self.last_col - self.first_col + 1
    }

    /// Returns true if this range contains only a single cell
    pub fn is_single_cell(&self) -> bool {
        self.first_row == self.last_row && self.first_col == self.last_col
    }

    /// Checks if this range contains the specified cell
    pub fn contains_cell(&self, row: i32, col: i32) -> bool {
        row >= self.first_row
            && row <= self.last_row
            && col >= self.first_col
            && col <= self.last_col
    }

    /// Checks if this range intersects with another range
    pub fn intersects(&self, other: &CellRangeAddress) -> bool {
        !(self.last_row < other.first_row
            || self.first_row > other.last_row
            || self.last_col < other.first_col
            || self.first_col > other.last_col)
    }

    /// Returns the intersection of this range with another range, if any
    pub fn intersection(&self, other: &CellRangeAddress) -> Option<Self> {
        if self.intersects(other) {
            Some(CellRangeAddress::new(
                self.first_row.max(other.first_row),
                self.last_row.min(other.last_row),
                self.first_col.max(other.first_col),
                self.last_col.min(other.last_col),
            ))
        } else {
            None
        }
    }

    /// Returns the union of this range with another range
    pub fn union(&self, other: &CellRangeAddress) -> Self {
        CellRangeAddress::new(
            self.first_row.min(other.first_row),
            self.last_row.max(other.last_row),
            self.first_col.min(other.first_col),
            self.last_col.max(other.last_col),
        )
    }
}

impl fmt::Display for CellRangeAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_as_string())
    }
}

impl FromStr for CellRangeAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CellRangeAddress::from_string(s)
    }
}

/// Converts Excel column letters to index (1-based)
fn column_letters_to_index(letters: &str) -> Result<i32, String> {
    let mut index = 0;
    for c in letters.chars() {
        if !c.is_ascii_alphabetic() {
            return Err(format!("Invalid character in column letters: {}", c));
        }

        let value = (c.to_ascii_uppercase() as u8 - b'A' + 1) as i32;
        index = index * 26 + value;

        // Excel maximum column is XFD (16384)
        if index > 16384 {
            return Err(format!("Column index exceeds Excel maximum: {}", index));
        }
    }

    Ok(index)
}

/// Converts index to Excel column letters (1-based)
fn index_to_column_letters(mut index: i32) -> String {
    let mut result = String::new();

    while index > 0 {
        let remainder = (index - 1) % 26;
        result.insert(0, (b'A' + remainder as u8) as char);
        index = (index - 1) / 26;
    }

    result
}

/// Formats a sheet name for display
fn format_sheet_name(sheet_name: &str) -> String {
    // If sheet name contains special characters, wrap in single quotes
    if sheet_name
        .contains(|c: char| c == ' ' || c == '\'' || c == '!' || c == ':' || c == '\\' || c == '/')
    {
        format!("'{}'", sheet_name.replace("'", "''"))
    } else {
        sheet_name.to_string()
    }
}

impl GenericRecord for CellRangeAddress {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        todo!()
    }
}
