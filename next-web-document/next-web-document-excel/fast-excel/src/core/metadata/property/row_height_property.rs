#[derive(Debug, Clone, Default)]
pub struct RowHeightProperty {
    height: u16,
}

impl RowHeightProperty {
    /// Creates a new instance with explicit coordinates.
    pub fn new(height: u16) -> Self {
        Self { height }
    }

    pub fn get_height(&self) -> u16 {
        self.height
    }

    pub fn set_height(&mut self, height: u16) {
        self.height = height;
    }
}
