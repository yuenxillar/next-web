#[derive(Debug, Clone)]
pub struct ColumnWidthProperty {
    width: Option<u32>,
}

impl ColumnWidthProperty {
    pub fn new(width: u32) -> Self {
        ColumnWidthProperty { width: Some(width) }
    }
}

impl ColumnWidthProperty {
    pub fn get_width(&self) -> Option<u32> {
        self.width
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = Some(width);
    }

    pub fn clear_width(&mut self) {
        self.width = None;
    }
}
