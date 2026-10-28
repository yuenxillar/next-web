#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CellType {
    /// No value / unset cell
    None = -1,

    /// Numeric value (integer or floating-point)
    Numeric = 0,

    /// String value
    String = 1,

    /// Cell contains a formula
    Formula = 2,

    /// Empty cell
    Blank = 3,

    /// Boolean value (true/false)
    Boolean = 4,

    /// Cell contains an error code
    Error = 5,
}

// Optional: convenient From<i32> if you still need to migrate old code
impl TryFrom<i32> for CellType {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, ()> {
        match value {
            -1 => Ok(CellType::None),
            0 => Ok(CellType::Numeric),
            1 => Ok(CellType::String),
            2 => Ok(CellType::Formula),
            3 => Ok(CellType::Blank),
            4 => Ok(CellType::Boolean),
            5 => Ok(CellType::Error),
            _ => Err(()),
        }
    }
}

// Optional: Display implementation for pretty printing
impl std::fmt::Display for CellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
