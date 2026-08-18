use axum::http::{StatusCode, header};
use reqwest::header::{
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD,
    ORIGIN, VARY,
};
use std::str::FromStr;
use tracing::{debug, trace};

use crate::{
    cors::{CorsConfiguration, CorsProcessor, CorsUtils},
    error::BoxError,
    http::HttpMethod,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

/// The Access-Control-Request-Private-Network request header field name.
const ACCESS_CONTROL_REQUEST_PRIVATE_NETWORK: &str = "Access-Control-Request-Private-Network";
/// The Access-Control-Allow-Private-Network response header field name.
const ACCESS_CONTROL_ALLOW_PRIVATE_NETWORK: &str = "Access-Control-Allow-Private-Network";

///
/// The default implementation of CorsProcessor, as defined by the CORS W3C recommendation  .
/// Note that when the supplied CorsConfiguration is null, this implementation does not reject CORS
/// requests outright but simply avoids adding CORS headers to the response. CORS processing is also
/// skipped if the response already contains CORS headers.
#[derive(Default)]
pub struct DefaultCorsProcessor;

impl DefaultCorsProcessor {
    /// Handle the internal CORS processing
    fn handle_internal(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        config: &CorsConfiguration,
        pre_flight_request: bool,
    ) -> Result<bool, BoxError> {
        // Get request origin
        let request_origin = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());

        // Check origin
        let allow_origin = match config.check_origin(request_origin)? {
            Some(origin) => origin,
            None => {
                debug!("Reject: '{:?}' origin is not allowed", request_origin);
                self.reject_request(response);
                return Ok(false);
            }
        };

        // Get method to use
        let request_method = Self::get_method_to_use(request, pre_flight_request);

        // Check methods
        let allow_methods = config.check_http_method(request_method.as_ref());
        if pre_flight_request && allow_methods.is_none() {
            debug!("Reject: HTTP '{:?}' is not allowed", request_method);
            self.reject_request(response);
            return Ok(false);
        }
        // Get headers to use
        let request_headers = Self::get_headers_to_use(request, pre_flight_request);

        // Check headers
        let allow_headers = config.check_headers(Some(&request_headers));
        if pre_flight_request && allow_headers.is_none() {
            debug!("Reject: headers '{:?}' are not allowed", request_headers);
            self.reject_request(response);
            return Ok(false);
        }

        // Set Access-Control-Allow-Origin
        response.insert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN.as_str(), &allow_origin);

        // Set methods for pre-flight
        if pre_flight_request {
            let methods_str = allow_methods
                .unwrap_or_default()
                .iter()
                .map(|m| m.as_ref())
                .collect::<Vec<_>>()
                .join(", ");
            response.insert_header(header::ACCESS_CONTROL_ALLOW_METHODS.as_str(), &methods_str);
        }

        let allow_headers = match allow_headers {
            Some(headers) => headers,
            None => return Ok(false),
        };
        // Set headers for pre-flight
        if pre_flight_request && !allow_headers.is_empty() {
            let headers_str = allow_headers.join(", ");
            response.insert_header(header::ACCESS_CONTROL_ALLOW_HEADERS.as_str(), &headers_str);
        }

        // Set exposed headers
        if let Some(exposed_headers) = config.exposed_headers().filter(|var| !var.is_empty()) {
            let exposed = exposed_headers.join(", ");
            response.insert_header(header::ACCESS_CONTROL_EXPOSE_HEADERS.as_str(), &exposed);
        }

        // Set allow credentials
        if config.allow_credentials().unwrap_or_default() {
            response.insert_header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS.as_str(), "true");
        }

        // Set private network
        if config.allow_private_network().unwrap_or_default() {
            if let Some(private_network) = request
                .headers()
                .get(ACCESS_CONTROL_REQUEST_PRIVATE_NETWORK)
            {
                if let Ok(value) = private_network.to_str() {
                    if value == "true" {
                        response.insert_header(ACCESS_CONTROL_ALLOW_PRIVATE_NETWORK, "true");
                    }
                }
            }
        }

        // Set max age for pre-flight
        if pre_flight_request {
            if let Some(max_age) = config.max_age() {
                response.insert_header(
                    header::ACCESS_CONTROL_MAX_AGE.as_str(),
                    &max_age.to_string(),
                );
            }
        }

        Ok(true)
    }

    /// Invoked when one of the CORS checks failed. The default implementation sets the response status
    /// to 403 and writes "Invalid CORS request" to the response.
    fn reject_request(&self, response: &mut dyn HttpResponse) {
        response.set_status_code(StatusCode::FORBIDDEN);
        response.set_body(b"Invalid CORS request".to_vec());
    }

    fn get_method_to_use(request: &dyn HttpRequest, is_pre_flight: bool) -> Option<HttpMethod> {
        if is_pre_flight {
            request
                .headers()
                .get(header::ACCESS_CONTROL_REQUEST_METHOD)
                .and_then(|v| v.to_str().ok())
                .and_then(|m| HttpMethod::from_str(m).ok())
        } else {
            Some(request.method())
        }
    }

    fn get_headers_to_use(request: &dyn HttpRequest, is_pre_flight: bool) -> Vec<String> {
        if is_pre_flight {
            request
                .headers()
                .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').map(|h| h.trim().to_string()).collect())
                .unwrap_or_default()
        } else {
            request
                .header_names()
                .into_iter()
                .map(ToString::to_string)
                .collect()
        }
    }
}

impl CorsProcessor for DefaultCorsProcessor {
    fn process_request(
        &self,
        configuration: Option<&CorsConfiguration>,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<bool, BoxError> {
        // If no config provided, skip CORS processing
        let config = match configuration {
            Some(config) => config,
            None => {
                if CorsUtils::is_cors_request(request) {
                    debug!("Skip: no CORS configuration has been provided");
                }
                return Ok(true);
            }
        };

        let vary_headers = response.headers(VARY.as_str()).unwrap_or_default();
        let mut headers_to_add = [
            (false, ORIGIN.as_str()),
            (false, ACCESS_CONTROL_REQUEST_METHOD.as_str()),
            (false, ACCESS_CONTROL_REQUEST_HEADERS.as_str()),
        ];

        if !vary_headers.contains(&ORIGIN.as_str()) {
            headers_to_add[0].0 = true;
        }

        if !vary_headers.contains(&ACCESS_CONTROL_REQUEST_METHOD.as_str()) {
            headers_to_add[1].0 = true;
        }

        if !vary_headers.contains(&ACCESS_CONTROL_REQUEST_HEADERS.as_str()) {
            headers_to_add[2].0 = true;
        }

        for (is_vary, name) in headers_to_add {
            if is_vary {
                response.append_header(VARY.as_str(), name);
            }
        }

        // Handle the request
        if !CorsUtils::is_cors_request(request) {
            return Ok(true);
        }

        if response
            .header(ACCESS_CONTROL_ALLOW_ORIGIN.as_str())
            .is_some()
        {
            trace!("Skip: response already contains \"Access-Control-Allow-Origin\"");
            return Ok(true);
        }

        self.handle_internal(
            request,
            response,
            config,
            CorsUtils::is_pre_flight_request(request),
        )
    }
}
