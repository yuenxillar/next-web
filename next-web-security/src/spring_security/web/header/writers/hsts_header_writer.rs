use tracing::{enabled, trace, Level};

use crate::web::header::HeaderWriter;

const HSTS_HEADER_NAME: &str = "Strict-Transport-Security";
const DEFAULT_MAX_AGE_SECONDS: u64 = 31536000;

pub struct HstsHeaderWriter {
    max_age_in_seconds: u64,
    include_sub_domains: bool,
    preload: bool,

    hsts_header_value: String,
}

impl HstsHeaderWriter {
    pub fn set_max_age_in_seconds(&mut self, max_age_in_seconds: u64) {
        self.max_age_in_seconds = max_age_in_seconds;
        self.update_hsts_header_value();
    }

    pub fn set_include_sub_domains(&mut self, include_sub_domains: bool) {
        self.include_sub_domains = include_sub_domains;
        self.update_hsts_header_value();
    }

    pub fn set_preload(&mut self, preload: bool) {
        self.preload = preload;
        self.update_hsts_header_value();
    }

    fn update_hsts_header_value(&mut self) {
        let mut hsts_header_value = String::from("max-age=");
        hsts_header_value.push_str(&self.max_age_in_seconds.to_string());

        if self.include_sub_domains {
            hsts_header_value += " ; includeSubDomains";
        }

        if self.preload {
            hsts_header_value += " ; preload";
        }

        self.hsts_header_value = hsts_header_value;
    }
}

impl HeaderWriter for HstsHeaderWriter {
    fn write_headers(
        &self,
        request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &mut dyn next_web_core::traits::http::http_response::HttpResponse,
    ) {
        if !request.is_secure() {
            if enabled!(Level::TRACE) {
                trace!("Not injecting HSTS header since it did not match request to [Is Secure]");
            }

            return;
        }

        if !response.contains_header(HSTS_HEADER_NAME) {
            response.insert_header(HSTS_HEADER_NAME, &self.hsts_header_value);
        }
    }
}

impl Default for HstsHeaderWriter {
    fn default() -> Self {
        let mut hsts_header_writer = Self {
            max_age_in_seconds: DEFAULT_MAX_AGE_SECONDS,
            include_sub_domains: true,
            preload: false,
            hsts_header_value: Default::default(),
        };
        hsts_header_writer.update_hsts_header_value();

        hsts_header_writer
    }
}
