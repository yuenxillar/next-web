#[derive(Debug, Clone, Default)]
pub struct OnceAbsoluteMergeProperty {
    /// First row
    first_row_index: u32,
    /// Last row
    last_row_index: u32,
    /// First column index
    first_column_index: u32,
    /// Last column index
    last_column_index: u32,
}

impl OnceAbsoluteMergeProperty {
    /// Creates a new instance with explicit coordinates.
    pub fn new(
        first_row_index: u32,
        last_row_index: u32,
        first_column_index: u32,
        last_column_index: u32,
    ) -> Self {
        Self {
            first_row_index,
            last_row_index,
            first_column_index,
            last_column_index,
        }
    }

    pub fn get_first_row_index(&self) -> u32 {
        self.first_row_index
    }

    pub fn get_last_row_index(&self) -> u32 {
        self.last_row_index
    }

    pub fn get_first_column_index(&self) -> u32 {
        self.first_column_index
    }

    pub fn get_last_column_index(&self) -> u32 {
        self.last_column_index
    }

    pub fn set_first_row_index(&mut self, first_row_index: u32) {
        self.first_row_index = first_row_index;
    }

    pub fn set_last_row_index(&mut self, last_row_index: u32) {
        self.last_row_index = last_row_index;
    }

    pub fn set_first_column_index(&mut self, first_column_index: u32) {
        self.first_column_index = first_column_index;
    }

    pub fn set_last_column_index(&mut self, last_column_index: u32) {
        self.last_column_index = last_column_index;
    }
}
