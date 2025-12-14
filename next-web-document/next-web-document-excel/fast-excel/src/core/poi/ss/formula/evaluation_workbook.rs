use crate::core::poi::ss::{
    formula::{
        evaluation_cell::EvaluationCell,
        evaluation_name::EvaluationName,
        evaluation_sheet::EvaluationSheet,
        ptg::{Ptg, name_ptg::NamePtg, name_x_ptg::NameXPtg},
    },
    spreadsheet_version::SpreadsheetVersion,
    usermodel::udff_inder::UDFFinder,
};

/// a workbook for the purpose of formula evaluation.
pub trait EvaluationWorkbook {
    /// Returns the name of the sheet at the given 0-based index.
    ///
    /// # Arguments
    /// * `sheet_index` - The 0-based index of the sheet
    ///
    /// # Returns
    /// The name of the sheet
    ///
    /// # Panics
    /// Panics if the index is outside the indices of available sheets
    fn get_sheet_name(&self, sheet_index: i32) -> String;

    /// Returns the sheet index for the given EvaluationSheet
    ///
    /// # Returns
    /// -1 if the specified sheet is from a different book
    fn get_sheet_index(&self, sheet: &dyn EvaluationSheet) -> i32;

    /// Finds a sheet index by case insensitive name.
    ///
    /// # Returns
    /// The index of the sheet matching the specified name. -1 if not found
    fn get_sheet_index_by_name(&self, sheet_name: &str) -> i32;

    /// Get the sheet identified by the given 0-based index.
    ///
    /// # Arguments
    /// * `sheet_index` - The 0-based index of the sheet
    ///
    /// # Returns
    /// The sheet
    ///
    /// # Panics
    /// Panics if the index is outside the indices of available sheets
    fn get_sheet(&self, sheet_index: i32) -> Box<dyn EvaluationSheet>;

    /// HSSF Only - fetch the external-style sheet details
    ///
    /// # Note
    /// Return will have no workbook set if it's actually in our own workbook
    ///
    /// # Returns
    /// The found sheet or None if not found
    ///
    /// # Panics
    /// Panics if called with XSSF or SXSSF workbooks
    fn get_external_sheet(&self, extern_sheet_index: i32) -> Option<ExternalSheet>;

    /// XSSF Only - fetch the external-style sheet details
    ///
    /// # Note
    /// Return will have no workbook set if it's actually in our own workbook
    ///
    /// # Returns
    /// The found sheet
    ///
    /// # Panics
    /// Panics if called with HSSF workbooks
    fn get_external_sheet_by_names(
        &self,
        first_sheet_name: &str,
        last_sheet_name: &str,
        external_workbook_number: i32,
    ) -> Option<ExternalSheetRange>;

    /// HSSF Only - convert an external sheet index to an internal sheet index,
    /// for an external-style reference to one of this workbook's own sheets
    fn convert_from_extern_sheet_index(&self, extern_sheet_index: i32) -> i32;

    /// HSSF Only - fetch the external-style name details
    fn get_external_name(
        &self,
        extern_sheet_index: i32,
        extern_name_index: i32,
    ) -> Option<ExternalName>;

    /// XSSF Only - fetch the external-style name details
    fn get_external_name_by_details(
        &self,
        name_name: &str,
        sheet_name: &str,
        external_workbook_number: i32,
    ) -> Option<ExternalName>;

    /// Get evaluation name by NamePtg
    fn get_name_by_ptg(&self, name_ptg: &NamePtg) -> Option<Box<dyn EvaluationName>>;

    /// Get evaluation name by name and sheet index
    fn get_name(&self, name: &str, sheet_index: i32) -> Option<Box<dyn EvaluationName>>;

    /// Resolve NameXPtg text
    fn resolve_name_x_text(&self, ptg: &NameXPtg) -> String;

    /// Get formula tokens for a cell
    fn get_formula_tokens(&self, cell: &dyn EvaluationCell) -> Vec<Ptg>;

    /// Get UDF finder
    fn get_udf_finder(&self) -> Option<&dyn UDFFinder>;

    /// Get spreadsheet version
    fn get_spreadsheet_version(&self) -> SpreadsheetVersion;

    /// Propagated from `WorkbookEvaluator::clear_all_cached_result_values()` to clear locally cached data.
    /// Implementations must call the same method on all referenced `EvaluationSheet` instances,
    /// as well as clearing local caches.
    ///
    /// # See Also
    /// `WorkbookEvaluator::clear_all_cached_result_values()`
    ///
    /// # Since
    /// POI 3.15 beta 3
    fn clear_all_cached_result_values(&mut self);
}

/// External sheet information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSheet {
    workbook_name: String,
    sheet_name: String,
}

impl ExternalSheet {
    /// Create a new ExternalSheet
    pub fn new(workbook_name: String, sheet_name: String) -> Self {
        Self {
            workbook_name,
            sheet_name,
        }
    }

    /// Get workbook name
    pub fn get_workbook_name(&self) -> &str {
        &self.workbook_name
    }

    /// Get sheet name
    pub fn get_sheet_name(&self) -> &str {
        &self.sheet_name
    }
}

/// External sheet range information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSheetRange {
    base: ExternalSheet,
    last_sheet_name: String,
}

impl ExternalSheetRange {
    /// Create a new ExternalSheetRange
    pub fn new(workbook_name: String, first_sheet_name: String, last_sheet_name: String) -> Self {
        Self {
            base: ExternalSheet::new(workbook_name, first_sheet_name),
            last_sheet_name,
        }
    }

    /// Get workbook name
    pub fn get_workbook_name(&self) -> &str {
        self.base.get_workbook_name()
    }

    /// Get first sheet name
    pub fn get_first_sheet_name(&self) -> &str {
        self.base.get_sheet_name()
    }

    /// Get last sheet name
    pub fn get_last_sheet_name(&self) -> &str {
        &self.last_sheet_name
    }
}

/// External name information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalName {
    name_name: String,
    name_number: i32,
    ix: i32,
}

impl ExternalName {
    /// Create a new ExternalName
    pub fn new(name_name: String, name_number: i32, ix: i32) -> Self {
        Self {
            name_name,
            name_number,
            ix,
        }
    }

    /// Get name
    pub fn get_name(&self) -> &str {
        &self.name_name
    }

    /// Get number
    pub fn get_number(&self) -> i32 {
        self.name_number
    }

    /// Get ix
    pub fn get_ix(&self) -> i32 {
        self.ix
    }
}
