#[derive(Debug, serde::Deserialize, Clone)]
pub struct HttpProperties {
    request: Option<RequestProperties>,
    response: Option<ResponseProperties>,
}

impl HttpProperties {
    pub fn new() -> Self {
        Self {
            request: None,
            response: None,
        }
    }
    pub fn request(&self) -> Option<&RequestProperties> {
        self.request.as_ref()
    }

    pub fn response(&self) -> Option<&ResponseProperties> {
        self.response.as_ref()
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct RequestProperties {
    // byte
    max_file_size: Option<usize>,
    // byte
    max_request_size: Option<usize>,
    trace: Option<bool>,
    location: Option<String>,

    /// from_secs
    timeout: Option<u64>,
}

impl RequestProperties {
    pub fn new() -> Self {
        Self {
            max_file_size: None,
            max_request_size: None,
            trace: Some(true),
            location: None,
            timeout: Some(5),
        }
    }

    pub fn max_file_size(&self) -> Option<usize> {
        self.max_file_size
    }

    pub fn max_request_size(&self) -> Option<usize> {
        self.max_request_size
    }

    pub fn trace(&self) -> bool {
        self.trace.unwrap_or(false)
    }

    pub fn timeout(&self) -> Option<u64> {
        self.timeout
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ResponseProperties {}

impl ResponseProperties {}
