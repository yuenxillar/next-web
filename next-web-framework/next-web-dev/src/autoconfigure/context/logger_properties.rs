#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoggerProperties {
    enable: Option<bool>,
    log_dir: Option<String>,
    log_maximum_size: Option<u32>,
    additional_date: Option<bool>,
}

impl LoggerProperties {
    pub fn enable(&self) -> bool {
        self.enable.unwrap_or(false)
    }

    pub fn log_dir(&self) -> Option<&str> {
        self.log_dir.as_deref()
    }

    pub fn log_maximum_size(&self) -> Option<u32> {
        self.log_maximum_size
    }

    pub fn additional_date(&self) -> bool {
        self.additional_date.unwrap_or(false)
    }
}
impl Default for LoggerProperties {
    fn default() -> Self {
        LoggerProperties {
            enable: None,
            log_dir: None,
            log_maximum_size: None,
            additional_date: None,
        }
    }
}
