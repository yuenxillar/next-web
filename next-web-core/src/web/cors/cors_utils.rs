use axum::http::{
    Uri,
    header::{ACCESS_CONTROL_REQUEST_METHOD, ORIGIN},
};

use crate::{http::HttpMethod, http::HttpRequest};

pub struct CorsUtils;

impl CorsUtils {
    pub fn is_cors_request(req: &dyn HttpRequest) -> bool {
        let origin = match req.header(ORIGIN.as_str()) {
            Some(origin) => origin,
            None => return false,
        };

        if origin.is_empty() {
            return false;
        }

        let origin_url: Uri = match origin.parse() {
            Ok(u) => u,
            Err(_) => return true, // malformed Origin → treat as CORS
        };

        // 3. Compare scheme, host, port
        let origin_scheme = origin_url.scheme().map(|s| s.as_str());
        let origin_host = origin_url.host();
        let origin_port = Self::get_port(origin_scheme, origin_url.port_u16());

        let server_scheme = req.scheme();
        let server_host = req.host();
        let server_port = Self::get_port(server_scheme, req.server_port());

        !(origin_scheme == server_scheme
            && origin_host == server_host
            && origin_port == server_port)
    }

    fn get_port(scheme: Option<&str>, port: Option<u16>) -> u16 {
        match port {
            None | Some(0) => match scheme {
                Some("http") | Some("ws") => 80,
                Some("https") | Some("wss") => 443,
                _ => 0,
            },
            Some(p) => p,
        }
    }

    pub fn is_pre_flight_request(request: &dyn HttpRequest) -> bool {
        request.method() == HttpMethod::OPTIONS
            && request.header(ORIGIN.as_str()).is_some()
            && request
                .header(ACCESS_CONTROL_REQUEST_METHOD.as_str())
                .is_some()
    }
}
