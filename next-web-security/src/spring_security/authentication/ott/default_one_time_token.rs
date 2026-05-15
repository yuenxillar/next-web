use chrono::{DateTime, Utc};

use super::one_time_token::OneTimeToken;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DefaultOneTimeToken {
    token: String,
    username: String,
    expires_at: DateTime<Utc>,
}

impl DefaultOneTimeToken {
    pub fn new(
        token: impl Into<String>,
        username: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        let token = token.into();
        let username = username.into();
        assert!(!token.trim().is_empty(), "token cannot be empty");
        assert!(!username.trim().is_empty(), "username cannot be empty");
        Self {
            token,
            username,
            expires_at,
        }
    }
}

impl OneTimeToken for DefaultOneTimeToken {
    fn token_value(&self) -> &str {
        &self.token
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}
