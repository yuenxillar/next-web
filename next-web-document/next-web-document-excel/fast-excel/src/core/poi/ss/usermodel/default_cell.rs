#[derive(Debug, Default, Clone)]
pub struct DefaultCell {
    row_index: Option<u32>,
    column_index: Option<u32>,
}

impl DefaultCell {
    pub fn new(row_index: u32, column_index: u32) -> Self {
        DefaultCell {
            row_index: Some(row_index),
            column_index: Some(column_index),
        }
    }

    pub fn get_row_index(&self) -> Option<u32> {
        self.row_index
    }

    pub fn get_column_index(&self) -> Option<u32> {
        self.column_index
    }

    pub fn set_row_index(&mut self, row_index: u32) {
        self.row_index = Some(row_index);
    }

    pub fn set_column_index(&mut self, column_index: u32) {
        self.column_index = Some(column_index);
    }
}
