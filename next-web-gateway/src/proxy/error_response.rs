//! Mapping of pingora errors to status codes, and rendering of the error body.

use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::ErrorType;

use crate::config::{ErrorConfig, ErrorFormat};

/// Status code for a proxy error, following the mapping in the design doc.
pub fn status_for_error(error: &pingora::Error) -> u16 {
    match error.etype() {
        ErrorType::HTTPStatus(status) => *status,
        ErrorType::ConnectTimedout
        | ErrorType::ReadTimedout
        | ErrorType::WriteTimedout
        | ErrorType::TLSHandshakeTimedout => 504,
        ErrorType::ConnectError
        | ErrorType::ConnectProxyFailure
        | ErrorType::TLSHandshakeFailure
        | ErrorType::ReadError
        | ErrorType::WriteError => 502,
        _ => match error.esource() {
            pingora::ErrorSource::Upstream => 502,
            pingora::ErrorSource::Downstream => match error.etype() {
                ErrorType::WriteError | ErrorType::ReadError | ErrorType::ConnectionClosed => 0,
                _ => 400,
            },
            pingora::ErrorSource::Internal | pingora::ErrorSource::Unset => 500,
        },
    }
}

/// Short reason phrase for a status code.
pub fn reason_phrase(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    }
}

/// Render the error response for `status`.
pub fn render_error(
    config: &ErrorConfig,
    status: u16,
    route: Option<&str>,
    request_id: &str,
) -> pingora::Result<(ResponseHeader, Bytes)> {
    let reason = reason_phrase(status);
    // Clamp before building: an out of range status would fail to build.
    let status = if (100..=599).contains(&status) {
        status
    } else {
        500
    };

    let mut header = ResponseHeader::build(status, None)?;

    header
        .insert_header("Content-Type", content_type(config))
        .ok();
    if !request_id.is_empty() {
        header
            .insert_header(config.request_id_header.clone(), request_id.to_string())
            .ok();
    }

    let body = match config.format {
        ErrorFormat::Json => {
            let route = route.unwrap_or("");
            Bytes::from(format!(
                "{{\"status\":{status},\"error\":\"{reason}\",\"route\":\"{route}\",\
                 \"requestId\":\"{request_id}\"}}"
            ))
        }
        ErrorFormat::Text => Bytes::from(format!("{status} {reason}\n")),
    };

    header
        .insert_header("Content-Length", body.len().to_string())
        .ok();

    Ok((header, body))
}

fn content_type(config: &ErrorConfig) -> &'static str {
    match config.format {
        ErrorFormat::Json => "application/json",
        ErrorFormat::Text => "text/plain; charset=utf-8",
    }
}

#[cfg(test)]
mod tests {
    use super::{reason_phrase, render_error, status_for_error};
    use crate::config::{ErrorConfig, ErrorFormat};

    #[test]
    fn maps_connect_failures_to_502_and_timeouts_to_504() {
        let connect = pingora::Error::new(pingora::ErrorType::ConnectError);
        assert_eq!(status_for_error(&connect), 502);

        let timeout = pingora::Error::new(pingora::ErrorType::ConnectTimedout);
        assert_eq!(status_for_error(&timeout), 504);

        let read_timeout = pingora::Error::new(pingora::ErrorType::ReadTimedout);
        assert_eq!(status_for_error(&read_timeout), 504);
    }

    #[test]
    fn keeps_an_explicit_http_status() {
        let error = pingora::Error::new(pingora::ErrorType::HTTPStatus(503));
        assert_eq!(status_for_error(&error), 503);
    }

    #[test]
    fn json_error_body_carries_request_id_and_route() {
        let config = ErrorConfig::default();
        let (header, body) = render_error(&config, 503, Some("users"), "abc123").unwrap();

        assert_eq!(header.status.as_u16(), 503);
        assert_eq!(
            header
                .headers
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        assert_eq!(
            header
                .headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("abc123")
        );

        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("\"status\":503"), "{body}");
        assert!(body.contains("\"route\":\"users\""), "{body}");
        assert!(body.contains("\"requestId\":\"abc123\""), "{body}");
    }

    #[test]
    fn text_error_body_is_a_single_line() {
        let config = ErrorConfig {
            format: ErrorFormat::Text,
            ..ErrorConfig::default()
        };
        let (_, body) = render_error(&config, 404, None, "abc123").unwrap();

        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "404 Not Found\n");
        assert_eq!(reason_phrase(404), "Not Found");
    }
}
