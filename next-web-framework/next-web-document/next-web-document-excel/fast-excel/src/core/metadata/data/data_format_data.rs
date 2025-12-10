/// data format
#[derive(Debug, Clone, Default)]
pub struct DataFormatData {
    index: Option<u16>,
    format: Option<String>,
}

impl DataFormatData {
    pub fn merge(source: &Self, target: &mut Self) {
        if let Some(index) = source.index {
            target.index = Some(index);
        }
        if let Some(format) = source.format.as_ref() {
            if !format.is_empty() {
                target.format = Some(format.clone());
            }
        }
    }

    pub fn get_index(&self) -> Option<u16> {
        self.index
    }

    pub fn get_format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    pub fn set_index(&mut self, index: u16) {
        self.index = Some(index);
    }

    pub fn set_format(&mut self, format: String) {
        self.format = Some(format);
    }

    pub fn clear_index(&mut self) {
        self.index = None;
    }

    pub fn clear_format(&mut self) {
        self.format = None;
    }
}
