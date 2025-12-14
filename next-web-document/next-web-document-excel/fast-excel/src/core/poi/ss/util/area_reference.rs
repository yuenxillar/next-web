use next_web_core::error::BoxError;

use crate::core::poi::ss::{
    spreadsheet_version::SpreadsheetVersion, util::cell_reference::CellReference,
};

/// Represents an area reference in a spreadsheet.
pub struct AreaReference {
    first_cell: CellReference,
    last_cell: CellReference,
    is_single_cell: bool,
    version: SpreadsheetVersion,
}

impl AreaReference {
    /// The character (!) that separates sheet names from cell references
    const SHEET_NAME_DELIMITER: char = '!';
    /// The character (:) that separates the two cell references in a multi-cell area reference
    const CELL_DELIMITER: char = ':';
    /// The character (') used to quote sheet names when they contain special characters
    const SPECIAL_NAME_DELIMITER: char = '\'';

    /// Create an area ref from a string representation.
    ///
    /// # Arguments
    /// * `reference` - String representation of the area reference
    /// * `version` - Spreadsheet version (uses Excel97 if None)
    ///
    /// # Panics
    /// Panics if the reference is not contiguous
    pub fn new(reference: &str, version: Option<SpreadsheetVersion>) -> Result<Self, BoxError> {
        let version = version.unwrap_or(SpreadsheetVersion::Excel97);

        if !Self::is_contiguous(reference) {
            panic!(
                "References passed to the AreaReference must be contiguous, \
                use generate_contiguous(ref) if you have non-contiguous references"
            );
        }

        let parts = Self::separate_area_refs(reference);
        let part0 = &parts[0];

        if parts.len() == 1 {
            // TODO - probably shouldn't initialize area ref when text is really a cell ref
            let first_cell = CellReference::new(part0)?;
            let last_cell = first_cell.clone();

            return Ok(Self {
                first_cell,
                last_cell,
                is_single_cell: true,
                version,
            });
        }

        if parts.len() != 2 {
            return Err(format!("Bad area ref '{}'", reference).into());
        }

        let part1 = &parts[1];
        if Self::is_plain_column(part0) {
            if !Self::is_plain_column(part1) {
                return Err(format!("Bad area ref '{}'", reference).into());
            }

            // Special handling for whole-column references
            let first_is_abs = CellReference::is_part_absolute(part0);
            let last_is_abs = CellReference::is_part_absolute(part1);
            let col0 = CellReference::convert_col_string_to_index(part0);
            let col1 = CellReference::convert_col_string_to_index(part1);

            let first_cell =
                CellReference::from_sheet_and_indices(None, 0, col0, true, first_is_abs);
            let last_cell =
                CellReference::from_sheet_and_indices(None, 0xFFFF, col1, true, last_is_abs);

            Ok(Self {
                first_cell,
                last_cell,
                is_single_cell: false,
                version,
            })
        } else {
            let first_cell = CellReference::new(part0)?;
            let last_cell = CellReference::new(part1)?;
            let is_single_cell = part0 == part1;

            Ok(Self {
                first_cell,
                last_cell,
                is_single_cell,
                version,
            })
        }
    }

    /// Creates an area ref from a pair of Cell References.
    pub fn from_cell_references(
        top_left: &CellReference,
        bot_right: &CellReference,
        version: Option<SpreadsheetVersion>,
    ) -> Self {
        let version = version.unwrap_or(SpreadsheetVersion::Excel97);
        let swap_rows = top_left.get_row() > bot_right.get_row();
        let swap_cols = top_left.get_col() > bot_right.get_col();

        if swap_rows || swap_cols {
            let (first_row, first_row_abs, last_row, last_row_abs) = if swap_rows {
                (
                    bot_right.get_row(),
                    bot_right.is_row_absolute(),
                    top_left.get_row(),
                    top_left.is_row_absolute(),
                )
            } else {
                (
                    top_left.get_row(),
                    top_left.is_row_absolute(),
                    bot_right.get_row(),
                    bot_right.is_row_absolute(),
                )
            };

            let (first_sheet, first_col, first_col_abs, last_sheet, last_col, last_col_abs) =
                if swap_cols {
                    (
                        bot_right.get_sheet_name().map(|s| s.to_string()),
                        bot_right.get_col(),
                        bot_right.is_col_absolute(),
                        top_left.get_sheet_name().map(|s| s.to_string()),
                        top_left.get_col(),
                        top_left.is_col_absolute(),
                    )
                } else {
                    (
                        top_left.get_sheet_name().map(|s| s.to_string()),
                        top_left.get_col(),
                        top_left.is_col_absolute(),
                        bot_right.get_sheet_name().map(|s| s.to_string()),
                        bot_right.get_col(),
                        bot_right.is_col_absolute(),
                    )
                };

            let first_cell = CellReference::from_sheet_and_indices(
                first_sheet,
                first_row,
                first_col,
                first_row_abs,
                first_col_abs,
            );
            let last_cell = CellReference::from_sheet_and_indices(
                last_sheet,
                last_row,
                last_col,
                last_row_abs,
                last_col_abs,
            );

            Self {
                first_cell,
                last_cell,
                is_single_cell: false,
                version,
            }
        } else {
            Self {
                first_cell: top_left.clone(),
                last_cell: bot_right.clone(),
                is_single_cell: false,
                version,
            }
        }
    }

    /// Checks if a string represents a plain column reference (e.g., "A", "B", "AA")
    fn is_plain_column(ref_part: &str) -> bool {
        for (i, ch) in ref_part.chars().rev().enumerate() {
            let pos = ref_part.len() - i - 1;
            if ch == '$' && pos == 0 {
                continue;
            }
            if !('A'..='Z').contains(&ch) {
                return false;
            }
        }
        true
    }

    /// Checks if the reference is for a contiguous area
    pub fn is_contiguous(reference: &str) -> bool {
        Self::split_area_references(reference).len() == 1
    }

    /// Construct an AreaReference which spans one or more rows
    pub fn get_whole_row(
        version: Option<SpreadsheetVersion>,
        start: &str,
        end: &str,
    ) -> Result<Self, BoxError> {
        let version = version.unwrap_or(SpreadsheetVersion::Excel97);
        let ref_str = format!("$A{}:${}{}", start, version.last_column_name(), end);
        Self::new(&ref_str, Some(version))
    }

    /// Construct an AreaReference which spans one or more columns
    pub fn get_whole_column(
        version: Option<SpreadsheetVersion>,
        start: &str,
        end: &str,
    ) -> Result<Self, BoxError> {
        let version = version.unwrap_or(SpreadsheetVersion::Excel97);
        let ref_str = format!("{}${}:{}${}", start, 1, end, version.max_rows());
        Self::new(&ref_str, Some(version))
    }

    /// Checks if the reference is for a whole-column reference
    pub fn is_whole_column_reference_with_params(
        version: Option<SpreadsheetVersion>,
        top_left: &CellReference,
        bot_right: &CellReference,
    ) -> bool {
        let version = version.unwrap_or(SpreadsheetVersion::Excel97);

        top_left.get_row() == 0
            && top_left.is_row_absolute()
            && bot_right.get_row() == version.last_row_index()
            && bot_right.is_row_absolute()
    }

    /// Takes a non-contiguous area reference, and returns an array of contiguous area references
    pub fn generate_contiguous(version: Option<SpreadsheetVersion>, reference: &str) -> Vec<Self> {
        let version = version.unwrap_or(SpreadsheetVersion::Excel97);
        let split_references = Self::split_area_references(reference);
        split_references
            .iter()
            .map(|ref_str| Self::new(ref_str, Some(version)))
            .collect()
    }

    /// Separates Area refs in two parts and returns them as separate elements in a vector
    fn separate_area_refs(reference: &str) -> Vec<String> {
        let len = reference.len();
        let mut delimiter_pos = -1_i32;
        let mut inside_delimited_name = false;

        for (i, ch) in reference.chars().enumerate() {
            match ch {
                Self::CELL_DELIMITER => {
                    if !inside_delimited_name {
                        if delimiter_pos >= 0 {
                            panic!(
                                "More than one cell delimiter '{}' appears in area reference '{}'",
                                Self::CELL_DELIMITER,
                                reference
                            );
                        }
                        delimiter_pos = i as i32;
                    }
                }
                Self::SPECIAL_NAME_DELIMITER => {
                    if !inside_delimited_name {
                        inside_delimited_name = true;
                    } else {
                        if i >= len - 1 {
                            panic!(
                                "Area reference '{}' ends with special name delimiter '{}'",
                                reference,
                                Self::SPECIAL_NAME_DELIMITER
                            );
                        }
                        if reference.chars().nth(i + 1) == Some(Self::SPECIAL_NAME_DELIMITER) {
                            // two consecutive quotes is the escape sequence for a single one
                            continue;
                        } else {
                            inside_delimited_name = false;
                        }
                    }
                }
                _ => {}
            }
        }

        if delimiter_pos < 0 {
            return vec![reference.to_string()];
        }

        let delimiter_pos = delimiter_pos as usize;
        let part_a = &reference[..delimiter_pos];
        let part_b = &reference[delimiter_pos + 1..];

        if part_b.contains(Self::SHEET_NAME_DELIMITER) {
            panic!(
                "Unexpected {} in second cell reference of '{}'",
                Self::SHEET_NAME_DELIMITER,
                reference
            );
        }

        if let Some(pling_pos) = part_a.rfind(Self::SHEET_NAME_DELIMITER) {
            let sheet_name = &part_a[..pling_pos + 1]; // +1 to include delimiter
            vec![part_a.to_string(), format!("{}{}", sheet_name, part_b)]
        } else {
            vec![part_a.to_string(), part_b.to_string()]
        }
    }

    /// Splits a comma-separated area references string into an array of individual references
    fn split_area_references(reference: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut current_segment = String::new();

        // Simple implementation - would need more sophisticated parsing for quoted references
        for part in reference.split(',') {
            if !current_segment.is_empty() {
                current_segment.push(',');
            }
            current_segment.push_str(part);

            // Simplified quote counting
            let single_quotes = current_segment.chars().filter(|&c| c == '\'').count();
            if single_quotes == 0 || single_quotes == 2 {
                results.push(current_segment.clone());
                current_segment.clear();
            }
        }

        if !current_segment.is_empty() {
            results.push(current_segment);
        }

        results
    }

    /// Checks if this is a whole column reference
    pub fn is_whole_column_reference(&self) -> bool {
        Self::is_whole_column_reference(Some(self.version), &self.first_cell, &self.last_cell)
    }

    /// Returns true if this area reference involves only one cell
    pub fn is_single_cell(&self) -> bool {
        self.is_single_cell
    }

    /// Returns the first cell reference which defines this area
    pub fn first_cell(&self) -> &CellReference {
        &self.first_cell
    }

    /// Returns the last cell reference which defines this area
    pub fn last_cell(&self) -> &CellReference {
        &self.last_cell
    }

    /// Returns a reference to every cell covered by this area
    pub fn all_referenced_cells(&self) -> Vec<CellReference> {
        if self.is_single_cell {
            return vec![self.first_cell.clone()];
        }

        let min_row = self.first_cell.get_row().min(self.last_cell.get_row());
        let max_row = self.first_cell.get_row().max(self.last_cell.get_row());
        let min_col = self.first_cell.get_col().min(self.last_cell.get_col());
        let max_col = self.first_cell.get_col().max(self.last_cell.get_col());
        let sheet_name = self.first_cell.get_sheet_name().map(|s| s.to_string());

        let mut refs = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let ref_cell = CellReference::from_sheet_and_indices(
                    sheet_name.clone(),
                    row,
                    col,
                    self.first_cell.is_row_absolute(),
                    self.first_cell.is_col_absolute(),
                );
                refs.push(ref_cell);
            }
        }
        refs
    }

    /// Returns a text representation of this area reference
    pub fn format_as_string(&self) -> String {
        // Special handling for whole-column references
        if self.is_whole_column_reference() {
            return format!(
                "{}:{}",
                CellReference::convert_num_to_col_string(self.first_cell.get_col()),
                CellReference::convert_num_to_col_string(self.last_cell.get_col())
            );
        }

        let mut result = String::with_capacity(32);
        result.push_str(&self.first_cell.format_as_string());

        if !self.is_single_cell {
            result.push(Self::CELL_DELIMITER);
            if self.last_cell.get_sheet_name().is_none() {
                result.push_str(&self.last_cell.format_as_string());
            } else {
                // Simplified - need to implement append_cell_reference in CellReference
                result.push_str(&self.last_cell.format_as_string());
            }
        }

        result
    }
}

impl std::fmt::Display for AreaReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AreaReference [{}]", self.format_as_string())
    }
}

impl std::fmt::Debug for AreaReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AreaReference")
            .field("first_cell", &self.first_cell)
            .field("last_cell", &self.last_cell)
            .field("is_single_cell", &self.is_single_cell)
            .field("version", &self.version)
            .finish()
    }
}
