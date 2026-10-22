//! Gateway level errors and their mapping to HTTP status codes.

use std::error::Error;

/// Errors the gateway raises itself, before or instead of an upstream call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum GatewayError {
    /// No route matched the request.
    ServerNoRoute,
    /// The matched route has no reachable upstream.
    ServerNoUpstreamServices,
    /// A filter rejected the request.
    ServerRejectsRequest,
    /// Anything else the gateway failed to do.
    ServerError,
}

impl GatewayError {
    /// Status code a client should see for this error.
    pub fn status(self) -> u16 {
        match self {
            Self::ServerNoRoute => 404,
            Self::ServerNoUpstreamServices => 503,
            Self::ServerRejectsRequest => 400,
            Self::ServerError => 500,
        }
    }
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GatewayError::ServerNoRoute => "No route matched the request",
            GatewayError::ServerRejectsRequest => "Server rejects the request",
            GatewayError::ServerError => "Server error",
            GatewayError::ServerNoUpstreamServices => "No upstream services available",
        })
    }
}

impl Error for GatewayError {}

impl From<GatewayError> for Box<pingora::Error> {
    fn from(error: GatewayError) -> Self {
        // A `HTTPStatus` error keeps its status through `status_for_error`, so
        // the client sees 404/503 instead of a generic 400/502.
        Box::new(*pingora::Error::create(
            pingora::ErrorType::HTTPStatus(error.status()),
            pingora::ErrorSource::Downstream,
            Some("GatewayError".into()),
            Some(Box::new(error)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayError;

    #[test]
    fn status_mapping_is_explicit() {
        assert_eq!(GatewayError::ServerNoRoute.status(), 404);
        assert_eq!(GatewayError::ServerNoUpstreamServices.status(), 503);
        assert_eq!(GatewayError::ServerRejectsRequest.status(), 400);
        assert_eq!(GatewayError::ServerError.status(), 500);
    }

    #[test]
    fn conversion_keeps_the_status() {
        let error: Box<pingora::Error> = GatewayError::ServerNoRoute.into();
        assert_eq!(error.etype(), &pingora::ErrorType::HTTPStatus(404));
    }
}
