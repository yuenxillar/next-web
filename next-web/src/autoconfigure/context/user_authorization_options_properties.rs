#[derive(Clone, Debug, serde::Deserialize)]

pub struct UserAuthorizationOptions {
    req_header: Option<String>,
    // s
    timeout: i64,
    concurrent: bool,
    share: bool,
    jwt_secret_key: Option<String>,
}

impl UserAuthorizationOptions {
    pub fn req_header(&self) -> Option<&str> {
        self.req_header.as_deref()
    }
    pub fn timeout(&self) -> i64 {
        self.timeout
    }
    pub fn concurrent(&self) -> bool {
        self.concurrent
    }
    pub fn share(&self) -> bool {
        self.share
    }
    pub fn jwt_secret_key(&self) -> Option<&str> {
        self.jwt_secret_key.as_deref()
    }
}

impl Default for UserAuthorizationOptions {
    fn default() -> Self {
        Self {
            req_header: Some("Authorization".to_string()),
            timeout: 3600 * 3,
            concurrent: true,
            share: true,
            jwt_secret_key: None,
        }
    }
}
