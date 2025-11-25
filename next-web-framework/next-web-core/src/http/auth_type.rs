use crate::traits::http::http_request::HttpRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthType {
    Basic,
    Digest,
    Bearer,

    Unknown(String),
    None,
}

impl AuthType {
    pub fn from_request(request: &dyn HttpRequest) -> Self {
        let auth_header = request.header("Authorization");
        match auth_header {
            Some(header) => match header {
                _ if header.starts_with("Basic ") => AuthType::Basic,
                _ if header.starts_with("Digest ") => AuthType::Digest,
                _ if header.starts_with("Bearer ") => AuthType::Bearer,
                _ => {
                    let (scheme, _) = header.split_once(' ').unwrap_or(("", ""));
                    if scheme.is_empty() {
                        AuthType::None
                    } else {
                        AuthType::Unknown(scheme.to_string())
                    }
                }
            },
            None => AuthType::None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            AuthType::Basic => "Basic",
            AuthType::Digest => "Digest",
            AuthType::Bearer => "Bearer",
            AuthType::None => "None",
            AuthType::Unknown(s) => s,
        }
    }
}
