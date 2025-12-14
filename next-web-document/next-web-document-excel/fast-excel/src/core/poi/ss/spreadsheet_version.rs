use crate::core::poi::ss::util::cell_reference::CellReference;

/// This enum allows spreadsheets from multiple Excel versions to be handled by the common code.
///
/// Properties of this enum correspond to attributes of the *spreadsheet* that are easily
/// discernable to the user. It is not intended to deal with low-level issues like file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpreadsheetVersion {
    /// Excel97 format aka BIFF8
    ///
    /// * The total number of available rows is 64k (2^16)
    /// * The total number of available columns is 256 (2^8)
    /// * The maximum number of arguments to a function is 30
    /// * Number of conditional format conditions on a cell is 3
    /// * Number of cell styles is 4000
    /// * Length of text cell contents is 32767
    Excel97,

    /// Excel2007
    ///
    /// * The total number of available rows is 1M (2^20)
    /// * The total number of available columns is 16K (2^14)
    /// * The maximum number of arguments to a function is 255
    /// * Number of conditional format conditions on a cell is unlimited
    ///   (actually limited by available memory in Excel)
    /// * Number of cell styles is 64000
    /// * Length of text cell contents is 32767
    Excel2007,
}

impl SpreadsheetVersion {
    const EXCEL97_MAX_ROWS: i32 = 0x10000; // 65536
    const EXCEL97_MAX_COLUMNS: i32 = 0x0100; // 256
    const EXCEL97_MAX_FUNCTION_ARGS: i32 = 30;
    const EXCEL97_MAX_COND_FORMATS: i32 = 3;
    const EXCEL97_MAX_CELL_STYLES: i32 = 4000;
    const EXCEL97_MAX_TEXT_LENGTH: i32 = 32767;

    const EXCEL2007_MAX_ROWS: i32 = 0x100000; // 1048576
    const EXCEL2007_MAX_COLUMNS: i32 = 0x4000; // 16384
    const EXCEL2007_MAX_FUNCTION_ARGS: i32 = 255;
    const EXCEL2007_MAX_COND_FORMATS: i32 = i32::MAX;
    const EXCEL2007_MAX_CELL_STYLES: i32 = 64000;
    const EXCEL2007_MAX_TEXT_LENGTH: i32 = 32767;

    /// Returns the maximum number of usable rows in each spreadsheet
    pub fn max_rows(&self) -> i32 {
        match self {
            SpreadsheetVersion::Excel97 => Self::EXCEL97_MAX_ROWS,
            SpreadsheetVersion::Excel2007 => Self::EXCEL2007_MAX_ROWS,
        }
    }

    /// Returns the last (maximum) valid row index, equals to `max_rows() - 1`
    pub fn last_row_index(&self) -> i32 {
        self.max_rows() - 1
    }

    /// Returns the maximum number of usable columns in each spreadsheet
    pub fn max_columns(&self) -> i32 {
        match self {
            SpreadsheetVersion::Excel97 => Self::EXCEL97_MAX_COLUMNS,
            SpreadsheetVersion::Excel2007 => Self::EXCEL2007_MAX_COLUMNS,
        }
    }

    /// Returns the last (maximum) valid column index, equals to `max_columns() - 1`
    pub fn last_column_index(&self) -> i32 {
        self.max_columns() - 1
    }

    /// Returns the maximum number arguments that can be passed to a multi-arg function (e.g. COUNTIF)
    pub fn max_function_args(&self) -> i32 {
        match self {
            SpreadsheetVersion::Excel97 => Self::EXCEL97_MAX_FUNCTION_ARGS,
            SpreadsheetVersion::Excel2007 => Self::EXCEL2007_MAX_FUNCTION_ARGS,
        }
    }

    /// Returns the maximum number of conditional format conditions on a cell
    pub fn max_conditional_formats(&self) -> i32 {
        match self {
            SpreadsheetVersion::Excel97 => Self::EXCEL97_MAX_COND_FORMATS,
            SpreadsheetVersion::Excel2007 => Self::EXCEL2007_MAX_COND_FORMATS,
        }
    }

    /// Returns the maximum number of cell styles per spreadsheet
    pub fn max_cell_styles(&self) -> i32 {
        match self {
            SpreadsheetVersion::Excel97 => Self::EXCEL97_MAX_CELL_STYLES,
            SpreadsheetVersion::Excel2007 => Self::EXCEL2007_MAX_CELL_STYLES,
        }
    }

    /// Returns the last valid column index in a ALPHA-26 representation (`IV` or `XFD`).
    pub fn last_column_name(&self) -> String {
        CellReference::convert_num_to_col_string(self.last_column_index())
    }

    /// Returns the maximum length of a text cell
    pub fn max_text_length(&self) -> i32 {
        match self {
            SpreadsheetVersion::Excel97 => Self::EXCEL97_MAX_TEXT_LENGTH,
            SpreadsheetVersion::Excel2007 => Self::EXCEL2007_MAX_TEXT_LENGTH,
        }
    }
}
