use crate::web::header::HeaderWriter;

const XFRAME_OPTIONS_HEADER: &str = "X-Frame-Options";

pub struct XFrameOptionsHeaderWriter {
    frame_options_mode: XFrameOptionsMode,
}

impl XFrameOptionsHeaderWriter {
    pub fn new(frame_options_mode: XFrameOptionsMode) -> Self {
        Self { frame_options_mode }
    }
}

impl HeaderWriter for XFrameOptionsHeaderWriter {
    fn write_headers(
        &self,
        _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        if XFrameOptionsMode::AllowFrom == self.frame_options_mode {
            return;
        } else {
            response.insert_header(XFRAME_OPTIONS_HEADER, self.frame_options_mode.get_mode());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XFrameOptionsMode {
    DenY,
    SameoriGin,
    AllowFrom,
}

impl XFrameOptionsMode {
    pub fn get_mode(&self) -> &'static str {
        match self {
            XFrameOptionsMode::DenY => "DENY",
            XFrameOptionsMode::SameoriGin => "SAMEORIGIN",
            XFrameOptionsMode::AllowFrom => "ALLOW-FROM",
        }
    }
}
