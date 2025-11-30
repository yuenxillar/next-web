pub struct ReadSheet {
    sheet_no: Option<u32>,
}

impl ReadSheet {
    pub fn set_sheet_no(&mut self, sheet_no: u32) {
        self.sheet_no = Some(sheet_no);
    }
}
impl Default for ReadSheet {
    fn default() -> Self {
        Self { sheet_no: None }
    }
}
