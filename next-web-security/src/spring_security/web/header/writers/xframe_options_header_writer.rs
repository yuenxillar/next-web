use crate::web::header::HeaderWriter;

const XFRAME_OPTIONS_HEADER: &str = "X-Frame-Options";

#[derive(Clone)]
pub struct XFrameOptionsHeaderWriter {
    frame_options_mode: XFrameOptionsMode,
}

impl XFrameOptionsHeaderWriter {
    /// Creates a new instance
    pub fn new(frame_options_mode: XFrameOptionsMode) -> Self {
        Self { frame_options_mode }
    }
}

impl HeaderWriter for XFrameOptionsHeaderWriter {
    /// Writes the X-Frame-Options header value, overwritting any previous value.
    fn write_headers(
        &self,
        _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        response.insert_header(XFRAME_OPTIONS_HEADER, self.frame_options_mode.get_mode());
    }
}

impl Default for XFrameOptionsHeaderWriter {
    fn default() -> Self {
        Self {
            frame_options_mode: XFrameOptionsMode::Deny,
        }
    }
}

/// The possible values for the X-Frame-Options header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XFrameOptionsMode {
    Deny,
    SameoriGin,
}

impl XFrameOptionsMode {
    pub fn get_mode(&self) -> &'static str {
        match self {
            XFrameOptionsMode::Deny => "DENY",
            XFrameOptionsMode::SameoriGin => "SAMEORIGIN",
        }
    }
}
