#[derive(Debug, Clone, Default)]
pub struct CoordinateData {
    /// first row index. Priority is higher than relative_first_row_index.
    first_row_index: Option<u32>,
    /// first column index. Priority is higher than relative_first_column_index.
    first_column_index: Option<u32>,
    /// last row index. Priority is higher than relative_last_row_index.
    last_row_index: Option<u32>,
    /// last column index. Priority is higher than relative_last_column_index.
    last_column_index: Option<u32>,
    /// relative first row index
    relative_first_row_index: Option<u32>,
    /// relative first column index
    relative_first_column_index: Option<u32>,
    /// relative last row index
    relative_last_row_index: Option<u32>,
    /// relative last column index
    relative_last_column_index: Option<u32>,
}

impl CoordinateData {
    pub fn get_first_row_index(&self) -> Option<u32> {
        self.first_row_index
    }

    pub fn get_first_column_index(&self) -> Option<u32> {
        self.first_column_index
    }

    pub fn get_last_row_index(&self) -> Option<u32> {
        self.last_row_index
    }

    pub fn get_last_column_index(&self) -> Option<u32> {
        self.last_column_index
    }

    pub fn get_relative_first_row_index(&self) -> Option<u32> {
        self.relative_first_row_index
    }

    pub fn get_relative_first_column_index(&self) -> Option<u32> {
        self.relative_first_column_index
    }

    pub fn get_relative_last_row_index(&self) -> Option<u32> {
        self.relative_last_row_index
    }

    pub fn get_relative_last_column_index(&self) -> Option<u32> {
        self.relative_last_column_index
    }

    pub fn set_first_row_index(&mut self, first_row_index: u32) {
        self.first_row_index = Some(first_row_index);
    }

    pub fn set_first_column_index(&mut self, first_column_index: u32) {
        self.first_column_index = Some(first_column_index);
    }

    pub fn set_last_row_index(&mut self, last_row_index: u32) {
        self.last_row_index = Some(last_row_index);
    }

    pub fn set_last_column_index(&mut self, last_column_index: u32) {
        self.last_column_index = Some(last_column_index);
    }

    pub fn set_relative_first_row_index(&mut self, relative_first_row_index: u32) {
        self.relative_first_row_index = Some(relative_first_row_index);
    }

    pub fn set_relative_first_column_index(&mut self, relative_first_column_index: u32) {
        self.relative_first_column_index = Some(relative_first_column_index);
    }

    pub fn set_relative_last_row_index(&mut self, relative_last_row_index: u32) {
        self.relative_last_row_index = Some(relative_last_row_index);
    }

    pub fn set_relative_last_column_index(&mut self, relative_last_column_index: u32) {
        self.relative_last_column_index = Some(relative_last_column_index);
    }

    pub fn clear_first_row_index(&mut self) {
        self.first_row_index = None;
    }

    pub fn clear_first_column_index(&mut self) {
        self.first_column_index = None;
    }

    pub fn clear_last_row_index(&mut self) {
        self.last_row_index = None;
    }

    pub fn clear_last_column_index(&mut self) {
        self.last_column_index = None;
    }

    pub fn clear_relative_first_row_index(&mut self) {
        self.relative_first_row_index = None;
    }

    pub fn clear_relative_first_column_index(&mut self) {
        self.relative_first_column_index = None;
    }

    pub fn clear_relative_last_row_index(&mut self) {
        self.relative_last_row_index = None;
    }

    pub fn clear_relative_last_column_index(&mut self) {
        self.relative_last_column_index = None;
    }
}
