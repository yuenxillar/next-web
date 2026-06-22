use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use reqwest::header::{
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD,
    ORIGIN, VARY,
};
use std::str::FromStr;
use tracing::{debug, trace};

use crate::{
    cors::{CorsConfiguration, CorsProcessor, CorsUtils},
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::http_method::HttpMethod,
};

/// Constants for CORS header fields
pub mod cors_headers {
    pub const ACCESS_CONTROL_REQUEST_PRIVATE_NETWORK: &str =
        "Access-Control-Request-Private-Network";
    pub const ACCESS_CONTROL_ALLOW_PRIVATE_NETWORK: &str = "Access-Control-Allow-Private-Network";
}

/// The default implementation of CORS processor
#[derive(Default)]
pub struct DefaultCorsProcessor;

impl DefaultCorsProcessor {
    /// Handle the internal CORS processing
    fn handle_internal(
        config: &CorsConfiguration,
        headers: &HeaderMap,
        method: &HttpMethod,
        pre_flight_request: bool,
    ) -> Result<Option<Response>, CorsError> {
        // Get request origin
        let request_origin = headers
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Check origin
        let allow_origin = config.check_origin(request_origin);
        if allow_origin.is_none() {
            debug!("Reject: '{}' origin is not allowed", request_origin);
            return Err(CorsError::OriginNotAllowed);
        }

        // Get method to use
        let request_method = Self::get_method_to_use(headers, method, pre_flight_request);

        // Check methods
        let allow_methods = config.check_http_method(&request_method);
        if allow_methods.is_none() {
            debug!("Reject: HTTP '{:?}' is not allowed", request_method);
            return Err(CorsError::MethodNotAllowed);
        }
        let allow_methods = allow_methods.unwrap();

        // Get headers to use
        let request_headers = Self::get_headers_to_use(headers, pre_flight_request);

        // Check headers
        let allow_headers = config.check_headers(&request_headers);
        if pre_flight_request && allow_headers.is_none() {
            debug!("Reject: headers '{:?}' are not allowed", request_headers);
            return Err(CorsError::HeadersNotAllowed);
        }

        // Build response with CORS headers
        let mut response = Response::builder().status(StatusCode::OK);

        // Set Access-Control-Allow-Origin
        response = response.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, allow_origin.unwrap());

        // Set methods for pre-flight
        if pre_flight_request {
            let methods_str = allow_methods
                .iter()
                .map(|m| m.as_ref())
                .collect::<Vec<_>>()
                .join(", ");
            response = response.header(header::ACCESS_CONTROL_ALLOW_METHODS, &methods_str);
        }

        // Set headers for pre-flight
        if pre_flight_request && !allow_headers.as_ref().unwrap().is_empty() {
            let headers_str = allow_headers.unwrap().join(", ");
            response = response.header(header::ACCESS_CONTROL_ALLOW_HEADERS, &headers_str);
        }

        // Set exposed headers
        if !config.exposed_headers.is_empty() {
            let exposed = config.exposed_headers.join(", ");
            response = response.header(header::ACCESS_CONTROL_EXPOSE_HEADERS, &exposed);
        }

        // Set allow credentials
        if config.allow_credentials {
            response = response.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        // Set private network
        if config.allow_private_network {
            if let Some(private_network) =
                headers.get(cors_headers::ACCESS_CONTROL_REQUEST_PRIVATE_NETWORK)
            {
                if let Ok(value) = private_network.to_str() {
                    if value == "true" {
                        response = response
                            .header(cors_headers::ACCESS_CONTROL_ALLOW_PRIVATE_NETWORK, "true");
                    }
                }
            }
        }

        // Set max age for pre-flight
        if pre_flight_request {
            if let Some(max_age) = config.max_age {
                response = response.header(header::ACCESS_CONTROL_MAX_AGE, max_age.to_string());
            }
        }

        let response = response
            .body(Body::empty())
            .map_err(|_| CorsError::InternalError)?;

        Ok(Some(response))
    }

    fn get_method_to_use(
        headers: &HeaderMap,
        method: &HttpMethod,
        is_pre_flight: bool,
    ) -> HttpMethod {
        if is_pre_flight {
            headers
                .get(header::ACCESS_CONTROL_REQUEST_METHOD)
                .and_then(|v| v.to_str().ok())
                .and_then(|m| HttpMethod::from_str(m).ok())
                .unwrap_or(HttpMethod::Options)
        } else {
            method.clone()
        }
    }

    fn get_headers_to_use(headers: &HeaderMap, is_pre_flight: bool) -> Vec<String> {
        if is_pre_flight {
            headers
                .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').map(|h| h.trim().to_string()).collect())
                .unwrap_or_default()
        } else {
            headers.keys().map(|k| k.as_str().to_string()).collect()
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
        // if !vary_headers.contains(&ORIGIN.as_str()) {
        //     response.append_header(VARY.as_str(), ORIGIN.as_str());
        // }

        // if !vary_headers.contains(&ACCESS_CONTROL_REQUEST_METHOD.as_str()) {
        //     response.append_header(VARY.as_str(), ACCESS_CONTROL_REQUEST_METHOD.as_str());
        // }

        // if !vary_headers.contains(&ACCESS_CONTROL_REQUEST_HEADERS.as_str()) {
        //     response.append_header(VARY.as_str(), ACCESS_CONTROL_REQUEST_HEADERS.as_str());
        // }

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

        // Self::handle_internal(config, headers, method, is_pre_flight)
        todo!()
    }
}

/// Error types for CORS processing
#[derive(Debug)]
pub enum CorsError {
    OriginNotAllowed,
    MethodNotAllowed,
    HeadersNotAllowed,
    InternalError,
}

impl std::fmt::Display for CorsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorsError::OriginNotAllowed => write!(f, "Origin not allowed"),
            CorsError::MethodNotAllowed => write!(f, "Method not allowed"),
            CorsError::HeadersNotAllowed => write!(f, "Headers not allowed"),
            CorsError::InternalError => write!(f, "Internal error"),
        }
    }
}

impl std::error::Error for CorsError {}
