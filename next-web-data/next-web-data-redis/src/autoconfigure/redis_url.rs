use std::fmt;

use redis::{ConnectionInfo, IntoConnectionInfo};

/// A parsed URL used to connect to Redis.
#[derive(Debug, Clone)]
pub struct RedisUrl {
    /// Original URL string (kept for reference / reconstruction).
    raw: String,
    use_ssl: bool,
    credentials: Credentials,
    database: u32,
}

impl RedisUrl {
    /// Parses the given Redis URL string.
    pub fn of(url: &str) -> Result<RedisUrl, RedisUrlSyntaxError> {
        let info: ConnectionInfo = url
            .into_connection_info()
            .map_err(|_| RedisUrlSyntaxError::new(url))?;

        let use_ssl = url.starts_with("rediss://");

        let credentials = Credentials::from_connection_info(&info);
        let database = info.redis_settings().db() as u32;

        Ok(RedisUrl {
            raw: url.to_string(),
            use_ssl,
            credentials,
            database,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn use_ssl(&self) -> bool {
        self.use_ssl
    }

    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    pub fn database(&self) -> u32 {
        self.database
    }
}

/// Redis connection credentials.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Credentials {
    username: Option<String>,
    password: Option<String>,
}

impl Credentials {
    const NONE: Credentials = Credentials {
        username: None,
        password: None,
    };

    /// Builds credentials from a parsed `ConnectionInfo`.
    fn from_connection_info(info: &ConnectionInfo) -> Credentials {
        let username = info.redis_settings().username().map(ToString::to_string);
        let password = info.redis_settings().password().map(ToString::to_string);
        if username.is_none() && password.is_none() {
            return Self::NONE;
        }
        Credentials { username, password }
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

/// Error returned when a Redis URL is invalid.
#[derive(Debug)]
pub struct RedisUrlSyntaxError {
    url: String,
}

impl RedisUrlSyntaxError {
    fn new(url: &str) -> Self {
        RedisUrlSyntaxError {
            url: url.to_string(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl fmt::Display for RedisUrlSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid Redis URL '{}'", self.url)
    }
}

impl std::error::Error for RedisUrlSyntaxError {}
