pub struct WriteSheet {
    /// Starting from 0
    sheet_no: Option<u32>,

    /// Sheet name
    sheet_name: Option<String>,
}

impl WriteSheet {
    pub fn new(sheet_no: u32, sheet_name: String) -> Self {
        Self {
            sheet_no: Some(sheet_no),
            sheet_name: Some(sheet_name),
        }
    }

    pub fn get_sheet_no(&self) -> Option<u32> {
        self.sheet_no
    }

    pub fn get_sheet_name(&self) -> Option<&str> {
        self.sheet_name.as_deref()
    }

    pub fn set_sheet_no(&mut self, sheet_no: u32) {
        self.sheet_no = Some(sheet_no);
    }

    pub fn set_sheet_name(&mut self, sheet_name: String) {
        self.sheet_name = Some(sheet_name);
    }
}

impl Default for WriteSheet {
    fn default() -> Self {
        Self {
            sheet_no: None,
            sheet_name: None,
        }
    }
}
