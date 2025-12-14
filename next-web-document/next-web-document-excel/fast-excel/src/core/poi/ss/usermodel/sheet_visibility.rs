/// Specifies sheet visibility
///
/// # See Also
/// * `Workbook::sheet_visibility()`
/// * `Workbook::set_sheet_visibility()`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetVisibility {
    /// Indicates the sheet is visible.
    Visible,
    /// Indicates the book window is hidden, but can be shown by the user via the user interface.
    Hidden,
    /// Indicates the sheet is hidden and cannot be shown in the user interface (UI).
    ///
    /// # Note
    /// In Excel this state is only available programmatically in VBA:
    /// `ThisWorkbook.Sheets("MySheetName").Visible = xlSheetVeryHidden`
    VeryHidden,
}
