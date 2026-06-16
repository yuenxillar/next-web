use std::fmt::Display;

use crate::web::header::HeaderWriter;

const XSS_PROTECTION_HEADER: &'static str = "X-XSS-Protection";

#[derive(Clone, Default)]
pub struct XXssProtectionHeaderWriter {
    header_value: XXssHeaderValue,
}

impl XXssProtectionHeaderWriter {
    pub fn set_header_value(&mut self, header_value: XXssHeaderValue) {
        self.header_value = header_value;
    }
}
impl HeaderWriter for XXssProtectionHeaderWriter {
    fn write_headers(
        &self,
        _request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        if !response.contains_header(XSS_PROTECTION_HEADER) {
            response.insert_header(XSS_PROTECTION_HEADER, self.header_value.as_ref());
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum XXssHeaderValue {
    #[default]
    Disabled,

    Enabled,
    EnabledModeBlock,
}

impl AsRef<str> for XXssHeaderValue {
    fn as_ref(&self) -> &str {
        match self {
            XXssHeaderValue::Disabled => "0",
            XXssHeaderValue::Enabled => "1",
            XXssHeaderValue::EnabledModeBlock => "1; mode=block",
        }
    }
}

impl Display for XXssProtectionHeaderWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "XXssProtectionHeaderWriter [header_value={}]",
            self.header_value.as_ref()
        )
    }
}
