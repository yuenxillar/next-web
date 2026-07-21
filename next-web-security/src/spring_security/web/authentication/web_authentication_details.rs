use std::fmt;

use next_web_core::traits::http::http_request::HttpRequest;

/// A holder of selected HTTP details related to a web authentication request.
#[derive(Clone, Debug)]
pub struct WebAuthenticationDetails {
    remote_address: String,
    session_id: Option<String>,
}

impl WebAuthenticationDetails {
    pub fn new(remote_address: impl Into<String>, session_id: Option<String>) -> Self {
        Self {
            remote_address: remote_address.into(),
            session_id,
        }
    }

    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    fn extract_session_id<'a>(request: &'a dyn HttpRequest) -> Option<&'a str> {
        request.session().map(|s| s.id())
    }
}

impl From<&dyn HttpRequest> for WebAuthenticationDetails {
    fn from(request: &dyn HttpRequest) -> Self {
        Self::new(
            request
                .remote_addr()
                .map(|r| r.to_string())
                .unwrap_or_default(),
            Self::extract_session_id(request).map(ToString::to_string),
        )
    }
}

impl fmt::Display for WebAuthenticationDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 获取类型名（类似 Java 的 getClass().getSimpleName()）
        let type_name = std::any::type_name::<Self>()
            .split(':')
            .last()
            .unwrap_or("WebAuthenticationDetails");

        write!(
            f,
            "{} [RemoteIpAddress={}, SessionId={}]",
            type_name,
            self.remote_address,
            self.session_id.as_deref().unwrap_or("null")
        )
    }
}
