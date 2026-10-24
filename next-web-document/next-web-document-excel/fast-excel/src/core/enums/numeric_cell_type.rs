/// Used to supplement CellType. Cannot distinguish between date and number in write case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericCellType {
    Number,

    /// date. Support only when writing.
    Date,
}
