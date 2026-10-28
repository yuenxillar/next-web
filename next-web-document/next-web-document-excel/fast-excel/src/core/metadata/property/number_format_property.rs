use bigdecimal::RoundingMode;

#[derive(Debug, Clone)]
pub struct NumberFormatProperty {
    format: Option<String>,
    rounding_mode: Option<RoundingMode>,
}

impl NumberFormatProperty {
    pub fn new(format: String, rounding_mode: RoundingMode) -> Self {
        NumberFormatProperty {
            format: Some(format),
            rounding_mode: Some(rounding_mode),
        }
    }

    pub fn get_format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    pub fn set_format(&mut self, format: String) {
        self.format = Some(format);
    }

    pub fn get_rounding_mode(&self) -> Option<&RoundingMode> {
        self.rounding_mode.as_ref()
    }

    pub fn set_rounding_mode(&mut self, rounding_mode: RoundingMode) {
        self.rounding_mode = Some(rounding_mode);
    }
}
