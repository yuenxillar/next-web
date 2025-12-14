use crate::core::poi::ss::formula::{
    evaluation_workbook::ExternalSheet,
    ptg::{name_ptg::NamePtg, name_x_ptg::NameXPtg},
};

/// Abstracts a workbook for the purpose of converting formula to text
pub trait FormulaRenderingWorkbook {
    /// Returns `None` if extern_sheet_index refers to a sheet inside the current workbook
    fn get_external_sheet(&self, extern_sheet_index: i32) -> Option<&ExternalSheet>;

    /// Returns the name of the (first) sheet referred to by the given external sheet index
    fn get_sheet_first_name_by_extern_sheet(&self, extern_sheet_index: i32) -> Option<&str>;

    /// Returns the name of the (last) sheet referred to by the given external sheet index
    fn get_sheet_last_name_by_extern_sheet(&self, extern_sheet_index: i32) -> Option<&str>;

    fn resolve_name_x_text(&self, name_x_ptg: &NameXPtg) -> String;
    fn get_name_text(&self, name_ptg: &NamePtg) -> Option<&str>;
}
