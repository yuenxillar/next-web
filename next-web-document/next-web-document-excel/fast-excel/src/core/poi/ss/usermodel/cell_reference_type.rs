/// Types of cell references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellReferenceType {
    /// Cells are referenced in the form A1, B4, etc.
    A1,

    /// Cells are referenced in the form R1C1, R4C2, etc.
    R1C1,

    /// The cell reference type is not defined explicitly by `A1` is the default in this case.
    Unknown,
}
