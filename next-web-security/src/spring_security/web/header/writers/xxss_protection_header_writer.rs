use crate::web::header::HeaderWriter;



const XSS_PROTECTION_HEADER: &'static str  = "X-XSS-Protection";

#[derive(Default)]
pub struct XXssProtectionHeaderWriter;

impl HeaderWriter for XXssProtectionHeaderWriter {
    fn write_headers(
        &self,
        _request: &mut dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        if !response.contains_header(XSS_PROTECTION_HEADER) {
            response.insert_header(XSS_PROTECTION_HEADER, "0");
        }
    }
}
