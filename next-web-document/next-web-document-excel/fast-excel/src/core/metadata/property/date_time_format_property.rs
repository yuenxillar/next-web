#[derive(Clone, Debug)]
pub struct DateTimeFormatProperty {
    format: Option<String>,
    use1904windowing: Option<bool>,
}

impl DateTimeFormatProperty {
    pub fn new<T: ToString>(format: T, use1904windowing: bool) -> Self {
        Self {
            format: Some(format.to_string()),
            use1904windowing: Some(use1904windowing),
        }
    }
}

impl DateTimeFormatProperty {
    pub fn get_format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    pub fn get_use1904windowing(&self) -> Option<bool> {
        self.use1904windowing
    }

    pub fn set_format(&mut self, format: String) {
        self.format = Some(format);
    }

    pub fn set_use1904windowing(&mut self, use1904windowing: bool) {
        self.use1904windowing = Some(use1904windowing);
    }

    pub fn clear_format(&mut self) {
        self.format = None;
    }

    pub fn clear_use1904windowing(&mut self) {
        self.use1904windowing = None;
    }
}
