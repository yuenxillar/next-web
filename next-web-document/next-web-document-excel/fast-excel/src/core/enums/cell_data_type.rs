#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellDataType {
    String,
    DirectString,
    Number,
    Boolean,
    Empty,
    Error,
    Date,
    RichTextString,
}

impl CellDataType {
    pub fn build_from_cell_type(cell_type: &str) -> Self {
        if cell_type.is_empty() {
            return Self::Empty;
        }

        Self::from(cell_type)
    }
}

impl From<&str> for CellDataType {
    fn from(cell_type: &str) -> Self {
        match cell_type {
            "s" => Self::String,
            "str" | "inlineStr" => Self::DirectString,
            "n" => Self::Number,
            "b" => Self::Boolean,
            "e" => Self::Error,
            _ => Self::Empty,
        }
    }
}
