use std::char;

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::core::poi::{
    common::usermodel::generic_record::GenericRecord,
    ss::{
        formula::sheet_name_formatter::SheetNameFormatter, spreadsheet_version::SpreadsheetVersion,
        util::generic_record_util::GenericRecordUtil,
    },
};

/// Matches a run of one or more letters followed by a run of one or more digits.
/// Both the letter and number groups are optional.
/// The run of letters is group 1 and the run of digits is group 2.
/// Each group may optionally be prefixed with a single '$'.
static CELL_REF_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(\$?[A-Z]+)?(\$?[0-9]+)?").unwrap());

/// Matches references only where row and column are included.
/// Matches a run of one or more letters followed by a run of one or more digits.
/// If a reference does not match this pattern, it might match COLUMN_REF_PATTERN or ROW_REF_PATTERN
/// References may optionally include a single '$' before each group, but these are excluded from the Matcher.group(int).
static STRICTLY_CELL_REF_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\$?([A-Z]+)\$?([0-9]+)").unwrap());

/// Matches a run of one or more letters. The run of letters is group 1.
/// References may optionally include a single '$' before the group, but these are excluded from the Matcher.group(int).
static COLUMN_REF_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\$?([A-Z]+)").unwrap());

/// Matches a run of one or more numbers. The run of numbers is group 1.
/// References may optionally include a single '$' before the group, but these are excluded from the Matcher.group(int).
static ROW_REF_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$?([0-9]+)").unwrap());

/// Named range names must start with a letter or underscore. Subsequent characters may include
/// digits or dot. (They can even end in dot).
static NAMED_RANGE_NAME_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)[_A-Z][_.A-Z0-9]*").unwrap());

/// Common conversion functions between Excel style A1, C27 style
/// cell references, and POI usermodel style row=0, column=0
/// style references. Handles sheet-based and sheet-free references
/// as well, eg "Sheet1!A1" and "$B$72"
///
/// Use `CellReference` when the concept of
/// relative/absolute does apply (such as a cell reference in a formula).
/// Use `CellAddress` when you want to refer to the location of a cell in a sheet
/// when the concept of relative/absolute does not apply (such as the anchor location
/// of a cell comment).
/// `CellReference`s have a concept of "sheet", while `CellAddress`es do not.
#[derive(Clone)]
pub struct CellReference {
    sheet_name: Option<String>,
    row_index: i32,
    col_index: i32,
    is_row_abs: bool,
    is_col_abs: bool,
}

impl CellReference {
    /// The character ($) that signifies a row or column value is absolute instead of relative
    const ABSOLUTE_REFERENCE_MARKER: char = '$';
    /// The character (!) that separates sheet names from cell references
    const SHEET_NAME_DELIMITER: char = '!';
    /// The character (') used to quote sheet names when they contain special characters
    const SPECIAL_NAME_DELIMITER: char = '\'';

    /// Create a cell ref from a string representation.
    /// Sheet names containing special characters should be delimited and escaped as per normal syntax rules for formulas.
    /// # Panics
    /// Panics if cell_ref is not valid
    pub fn new(cell_ref: &str) -> Result<Self, String> {
        if cell_ref.ends_with("#REF!") {
            return Err(format!("Cell reference invalid: {}", cell_ref));
        }

        let parts = Self::separate_ref_parts(cell_ref);

        let mut col_ref = parts.col_ref;
        let is_col_abs = !col_ref.is_empty() && col_ref.starts_with('$');
        if is_col_abs {
            col_ref = col_ref[1..].to_string();
        }
        let col_index = if col_ref.is_empty() {
            -1
        } else {
            Self::convert_col_string_to_index(&col_ref)
        };

        let mut row_ref = parts.row_ref;
        let is_row_abs = !row_ref.is_empty() && row_ref.starts_with('$');
        if is_row_abs {
            row_ref = row_ref[1..].to_string();
        }
        let row_index = if row_ref.is_empty() {
            -1
        } else {
            row_ref.parse::<i32>().unwrap() - 1 // -1 to convert 1-based to zero-based
        };

        Ok(Self {
            sheet_name: parts.sheet_name,
            row_index,
            col_index,
            is_row_abs,
            is_col_abs,
        })
    }

    /// Create a cell reference with row and column indices
    pub fn from_indices(row: i32, col: i32) -> Self {
        Self::from_indices_with_abs(row, col, false, false)
    }

    /// Create a cell reference with row and column indices and absolute flags
    pub fn from_indices_with_abs(row: i32, col: i32, abs_row: bool, abs_col: bool) -> Self {
        Self::from_sheet_and_indices(None, row, col, abs_row, abs_col)
    }

    /// Create a cell reference with sheet name and indices
    pub fn from_sheet_and_indices(
        sheet_name: Option<String>,
        row: i32,
        col: i32,
        abs_row: bool,
        abs_col: bool,
    ) -> Self {
        if row < -1 {
            panic!("row index may not be negative, but had {}", row);
        }
        if col < -1 {
            panic!("column index may not be negative, but had {}", col);
        }

        Self {
            sheet_name,
            row_index: row,
            col_index: col,
            is_row_abs: abs_row,
            is_col_abs: abs_col,
        }
    }

    /// Returns the row index (0-based)
    pub fn get_row(&self) -> i32 {
        self.row_index
    }

    /// Returns the column index (0-based)
    pub fn get_col(&self) -> i32 {
        self.col_index
    }

    /// Returns true if the row reference is absolute
    pub fn is_row_absolute(&self) -> bool {
        self.is_row_abs
    }

    /// Returns true if the column reference is absolute
    pub fn is_col_absolute(&self) -> bool {
        self.is_col_abs
    }

    /// Returns the sheet name, or None if this is a 2D reference.
    /// Special characters are not escaped or delimited
    pub fn get_sheet_name(&self) -> Option<&str> {
        self.sheet_name.as_deref()
    }

    /// Checks if a reference part is absolute
    pub fn is_part_absolute(part: &str) -> bool {
        part.starts_with(Self::ABSOLUTE_REFERENCE_MARKER)
    }

    /// Converts column reference portion from ALPHA-26 format to 0-based base 10.
    /// 'A' -> 0, 'Z' -> 25, 'AA' -> 26, 'IV' -> 255
    pub fn convert_col_string_to_index(ref_str: &str) -> i32 {
        let mut retval = 0;
        let ref_chars = ref_str.to_uppercase().chars().collect::<Vec<char>>();

        for (k, ch) in ref_chars.iter().enumerate() {
            if *ch == Self::ABSOLUTE_REFERENCE_MARKER {
                if k != 0 {
                    panic!("Bad col ref format '{}'", ref_str);
                }
                continue;
            }

            // Character is uppercase letter, find relative value to A
            retval = (retval * 26) + (*ch as u32 - 'A' as u32 + 1);
        }
        (retval - 1) as i32
    }

    /// Classifies an identifier as either a simple (2D) cell reference or a named range name
    pub fn classify_cell_reference(str_ref: &str, ss_version: SpreadsheetVersion) -> NameType {
        if str_ref.is_empty() {
            panic!("Empty string not allowed");
        }

        let first_char = str_ref.chars().next().unwrap();
        match first_char {
            Self::ABSOLUTE_REFERENCE_MARKER | '.' | '_' => {}
            _ => {
                if !first_char.is_ascii_alphabetic() && !first_char.is_ascii_digit() {
                    panic!(
                        "Invalid first char ({}) of cell reference or named range. Letter expected",
                        first_char
                    );
                }
            }
        }

        if !str_ref.chars().last().unwrap().is_ascii_digit() {
            return Self::validate_named_range_name(str_ref, ss_version);
        }

        // Note: Regex patterns would need to be implemented with regex crate
        // For simplicity, we'll implement the logic without regex for now
        Self::validate_cell_reference(str_ref, ss_version)
    }

    /// Helper method to validate cell references
    fn validate_cell_reference(str: &str, ss_version: SpreadsheetVersion) -> NameType {
        let caps = match STRICTLY_CELL_REF_PATTERN.captures(str) {
            Some(caps) => caps,
            None => return Self::validate_named_range_name(str, ss_version),
        };

        let letters_group = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let digits_group = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if Self::cell_reference_is_within_range(letters_group, digits_group, ss_version) {
            // valid cell reference
            return NameType::Cell;
        }

        // If str looks like a cell reference, but is out of (row/col) range, it is a valid
        // named range name
        // This behaviour is a little weird.  For example, "IW123" is a valid named range name
        // because the column "IW" is beyond the maximum "IV".  Note - this behaviour is version
        // dependent.  In BIFF12, "IW123" is not a valid named range name, but in BIFF8 it is.
        if str
            .find(Self::ABSOLUTE_REFERENCE_MARKER)
            .unwrap_or_default()
            >= 0
        {
            // Of course, named range names cannot have '$'
            return NameType::BadCellOrNamedRange;
        }

        NameType::NamedRange
    }

    /// Helper method to validate named range names
    fn validate_named_range_name(str: &str, ss_version: SpreadsheetVersion) -> NameType {
        // 检查是否为列引用（仅字母）
        if let Some(caps) = COLUMN_REF_PATTERN.captures(str) {
            let col_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if Self::is_column_within_range(col_str, ss_version) {
                return NameType::Column;
            }
        }

        // 检查是否为行引用（仅数字）
        if let Some(caps) = ROW_REF_PATTERN.captures(str) {
            let row_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if Self::is_row_within_range(row_str, ss_version) {
                return NameType::Row;
            }
        }

        // 检查是否符合命名范围名称模式
        if !NAMED_RANGE_NAME_PATTERN.is_match(str) {
            return NameType::BadCellOrNamedRange;
        }

        NameType::NamedRange
    }

    /// Checks if a cell reference is within the spreadsheet range
    pub fn cell_reference_is_within_range(
        col_str: &str,
        row_str: &str,
        ss_version: SpreadsheetVersion,
    ) -> bool {
        if !Self::is_column_within_range(col_str, ss_version) {
            return false;
        }
        Self::is_row_within_range(row_str, ss_version)
    }

    /// Checks if a column reference is within the spreadsheet range
    pub fn is_column_within_range(col_str: &str, ss_version: SpreadsheetVersion) -> bool {
        let last_col = ss_version.last_column_name();
        let last_col_len = last_col.len();
        let col_len = col_str.len();

        if col_len > last_col_len {
            return false;
        }

        if col_len == last_col_len {
            if col_str.to_uppercase() > last_col {
                return false;
            }
        }

        true
    }

    /// Checks if a row reference is within the spreadsheet range
    pub fn is_row_within_range(row_str: &str, ss_version: SpreadsheetVersion) -> bool {
        let row_num = row_str.parse::<i64>().unwrap() - 1;
        if row_num > i32::MAX as i64 {
            return false;
        }
        Self::is_row_within_range_index(row_num as i32, ss_version)
    }

    /// Checks if a row index is within the spreadsheet range
    pub fn is_row_within_range_index(row_num: i32, ss_version: SpreadsheetVersion) -> bool {
        row_num >= 0 && row_num <= ss_version.last_row_index()
    }

    /// Separates the sheet name, row, and columns from a cell reference string
    fn separate_ref_parts(reference: &str) -> CellRefParts {
        let pling_pos = reference.rfind(Self::SHEET_NAME_DELIMITER);
        let sheet_name = Self::parse_sheet_name(reference, pling_pos);

        let cell_start = pling_pos.map(|p| p + 1).unwrap_or(0);
        let cell = reference[cell_start..].to_uppercase();

        let caps = match CELL_REF_PATTERN.captures(&cell) {
            Some(caps) => caps,
            None => panic!("Invalid CellReference: {}", reference),
        };

        let col = caps.get(1).map(|s| s.as_str()).map(ToString::to_string);
        let row = caps.get(2).map(|s| s.as_str()).map(ToString::to_string);

        CellRefParts {
            sheet_name,
            row_ref: row.unwrap_or_default(),
            col_ref: col.unwrap_or_default(),
        }
    }

    /// Parses sheet name from reference
    fn parse_sheet_name(reference: &str, index_of_delimiter: Option<usize>) -> Option<String> {
        let delimiter_pos = match index_of_delimiter {
            Some(pos) => pos,
            None => return None,
        };

        let is_quoted = reference.starts_with(Self::SPECIAL_NAME_DELIMITER);
        if !is_quoted {
            // sheet names with spaces must be quoted
            if !reference.contains(' ') {
                return Some(reference[..delimiter_pos].to_string());
            } else {
                panic!(
                    "Sheet names containing spaces must be quoted: ({})",
                    reference
                );
            }
        }

        let last_quote_pos = delimiter_pos - 1;
        if !reference[last_quote_pos..].starts_with(Self::SPECIAL_NAME_DELIMITER) {
            panic!("Mismatched quotes: ({})", reference);
        }

        let mut result = String::with_capacity(delimiter_pos);
        let chars: Vec<char> = reference.chars().collect();

        for i in 1..last_quote_pos {
            let ch = chars[i];
            if ch != Self::SPECIAL_NAME_DELIMITER {
                result.push(ch);
                continue;
            }

            if i + 1 < last_quote_pos && chars[i + 1] == Self::SPECIAL_NAME_DELIMITER {
                // two consecutive quotes is the escape sequence for a single one
                result.push(ch);
                // Note: would need to increment i here in a loop
                continue;
            }

            panic!("Bad sheet name quote escaping: ({})", reference);
        }

        Some(result)
    }

    /// Converts 0-based column index to ALPHA-26 representation
    pub fn convert_num_to_col_string(col: i32) -> String {
        // Excel counts column A as the 1st column, we
        //  treat it as the 0th one
        let excel_col_num = col + 1;
        let mut col_remain = excel_col_num;
        let mut result = String::new();

        while col_remain > 0 {
            let this_part = col_remain % 26;
            let this_part = if this_part == 0 { 26 } else { this_part };
            col_remain = (col_remain - this_part) / 26;

            let col_char = ((this_part as u8) + 64) as char;
            result.insert(0, col_char);
        }

        result
    }

    /// Returns a text representation of this cell reference
    pub fn format_as_string(&self) -> String {
        self.format_as_string_with_sheet(true)
    }

    /// Returns a text representation in R1C1 format
    pub fn format_as_r1c1_string(&self) -> String {
        self.format_as_r1c1_string_with_sheet(true)
    }

    /// Internal implementation of format_as_string
    fn format_as_string_with_sheet(&self, include_sheet_name: bool) -> String {
        let mut result = String::with_capacity(32);

        if include_sheet_name {
            if let Some(sheet_name) = &self.sheet_name {
                SheetNameFormatter::append_format(&mut result, sheet_name);
                result.push(Self::SHEET_NAME_DELIMITER);
            }
        }

        self.append_cell_reference(&mut result);
        result
    }

    /// Internal implementation of format_as_r1c1_string
    pub fn format_as_r1c1_string_with_sheet(&self, include_sheet_name: bool) -> String {
        let mut result = String::with_capacity(32);

        if include_sheet_name {
            if let Some(sheet_name) = &self.sheet_name {
                SheetNameFormatter::append_format(&mut result, sheet_name.as_str());
                result.push(Self::SHEET_NAME_DELIMITER);
            }
        }

        self.append_r1c1_cell_reference(&mut result);
        result
    }

    /// Returns the three parts of the cell reference
    pub fn get_cell_ref_parts(&self) -> [String; 3] {
        [
            self.get_sheet_name()
                .map(ToString::to_string)
                .unwrap_or_default(),
            (self.row_index + 1).to_string(),
            Self::convert_num_to_col_string(self.col_index),
        ]
    }

    /// Appends cell reference with '$' markers for absolute values
    fn append_cell_reference(&self, sb: &mut String) {
        if self.col_index != -1 {
            if self.is_col_abs {
                sb.push(Self::ABSOLUTE_REFERENCE_MARKER);
            }
            sb.push_str(&Self::convert_num_to_col_string(self.col_index));
        }
        if self.row_index != -1 {
            if self.is_row_abs {
                sb.push(Self::ABSOLUTE_REFERENCE_MARKER);
            }
            sb.push_str(&(self.row_index + 1).to_string());
        }
    }

    /// Appends R1C1 cell reference with '$' markers for absolute values
    fn append_r1c1_cell_reference(&self, sb: &mut String) {
        if self.row_index != -1 {
            sb.push('R');
            sb.push_str(&(self.row_index + 1).to_string());
        }
        if self.col_index != -1 {
            sb.push('C');
            sb.push_str(&(self.col_index + 1).to_string());
        }
    }
}

impl std::fmt::Display for CellReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CellReference [{}]", self.format_as_string())
    }
}

impl std::fmt::Debug for CellReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellReference")
            .field("sheet_name", &self.sheet_name)
            .field("row_index", &self.row_index)
            .field("col_index", &self.col_index)
            .field("is_row_abs", &self.is_row_abs)
            .field("is_col_abs", &self.is_col_abs)
            .finish()
    }
}

impl PartialEq for CellReference {
    fn eq(&self, other: &Self) -> bool {
        self.row_index == other.row_index
            && self.col_index == other.col_index
            && self.is_row_abs == other.is_row_abs
            && self.is_col_abs == other.is_col_abs
            && self.sheet_name == other.sheet_name
    }
}

impl Eq for CellReference {}

impl std::hash::Hash for CellReference {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.row_index.hash(state);
        self.col_index.hash(state);
        self.is_row_abs.hash(state);
        self.is_col_abs.hash(state);
        self.sheet_name.hash(state);
    }
}

/// Internal struct for separating reference parts
struct CellRefParts {
    sheet_name: Option<String>,
    row_ref: String,
    col_ref: String,
}

/// Used to classify identifiers found in formulas as cell references or not.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameType {
    Cell,
    NamedRange,
    Column,
    Row,
    BadCellOrNamedRange,
}

impl GenericRecord for CellReference {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        let properties = GenericRecordUtil::get_generic_properties6(
            "sheetName",
            AnyValue::String(
                self.get_sheet_name()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            ),
            "rowIndex",
            AnyValue::Number(self.row_index as i64),
            "colIndex",
            AnyValue::Number(self.col_index as i64),
            "rowAbs",
            AnyValue::Boolean(self.is_row_abs),
            "colAbs",
            AnyValue::Boolean(self.is_col_abs),
            "formatAsString",
            AnyValue::String(self.format_as_string()),
        );
        Some(properties)
    }
}
