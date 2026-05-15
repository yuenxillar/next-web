use chrono::{DateTime, Utc};

pub trait OneTimeToken: Send + Sync {
    fn token_value(&self) -> &str;

    fn username(&self) -> &str;

    fn expires_at(&self) -> DateTime<Utc>;
}
