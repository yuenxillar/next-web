pub struct WebAttributes;

impl WebAttributes {
    /// Used to cache an AccessDeniedError in the request for rendering.
    pub const ACCESS_DENIED_403: &str = "NEXT_SECURITY_403_ERROR";

    pub const REQUIRED_FACTOR_ERRORS: &str = "web.WebAttributes.REQUIRED_FACTOR_ERRORS";

    pub const AUTHENTICATION_ERROR: &str = "NEXT_SECURITY_LAST_ERROR";
}
