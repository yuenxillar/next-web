use chrono::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerateOneTimeTokenRequest {
    username: String,
    expires_in: Duration,
}

impl GenerateOneTimeTokenRequest {
    pub fn new(username: impl Into<String>) -> Self {
        Self::with_expires_in(username, Duration::minutes(5))
    }

    pub fn with_expires_in(username: impl Into<String>, expires_in: Duration) -> Self {
        let username = username.into();
        Self {
            username,
            expires_in,
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn expires_in(&self) -> Duration {
        self.expires_in
    }
}
