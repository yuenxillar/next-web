use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for HttpProperties {
    fn default() -> Self {
        Self {
            request: Some(Default::default()),
            response: Some(Default::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestProperties {
    /// byte
    max_file_size: Option<usize>,
    /// byte
    max_request_size: Option<usize>,
    /// enable trace layer
    trace: bool,
    location: Option<String>,

    /// http request timeout, from seconds
    #[serde(default = "default_timeout")]
    timeout: u64,
}

impl RequestProperties {
    pub fn new(
        max_file_size: usize,
        max_request_size: usize,
        trace: bool,
        location: String,
        timeout: u64,
    ) -> Self {
        Self {
            max_file_size: Some(max_file_size),
            max_request_size: Some(max_request_size),
            trace,
            location: Some(location),
            timeout,
        }
    }

    pub fn max_file_size(&self) -> Option<usize> {
        self.max_file_size
    }

    pub fn max_request_size(&self) -> Option<usize> {
        self.max_request_size
    }

    pub fn trace(&self) -> bool {
        self.trace
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}

fn default_timeout() -> u64 {
    5
}

impl Default for RequestProperties {
    fn default() -> Self {
        Self {
            max_file_size: None,
            max_request_size: None,
            trace: false,
            location: None,
            timeout: default_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseProperties {}

impl ResponseProperties {}

impl Default for ResponseProperties {
    fn default() -> Self {
        Self {}
    }
}
