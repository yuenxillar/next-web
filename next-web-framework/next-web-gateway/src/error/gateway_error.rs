use std::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayError {
    ServerRejectsRequest,
    ServerNoUpstreamServices,
    ServerError,
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GatewayError::ServerRejectsRequest => "Server rejects the request",
            GatewayError::ServerError => "Server error",
            GatewayError::ServerNoUpstreamServices => "No upstream services available",
        })
    }
}


impl Error for GatewayError {}

impl Into<Box<pingora::Error>> for GatewayError {
    fn into(self) -> Box<pingora::Error> {
        Box::new(*pingora::Error::create(
            pingora::ConnectProxyFailure,
            pingora::ErrorSource::Downstream,
            Some("GatewayError".into()),
            Some(Box::new(self)),
        ))
    }
}
