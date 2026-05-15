use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionInformation {
    last_request: DateTime<Utc>,
    principal: String,
    session_id: String,
    expired: bool,
}

impl SessionInformation {
    pub fn new(
        principal: impl Into<String>,
        session_id: impl Into<String>,
        last_request: DateTime<Utc>,
    ) -> Self {
        let principal = principal.into();
        let session_id = session_id.into();
        assert!(!principal.trim().is_empty(), "Principal required");
        assert!(!session_id.trim().is_empty(), "SessionId required");
        Self {
            last_request,
            principal,
            session_id,
            expired: false,
        }
    }

    pub fn expire_now(&mut self) {
        self.expired = true;
    }

    pub fn last_request(&self) -> DateTime<Utc> {
        self.last_request
    }

    pub fn principal(&self) -> &str {
        &self.principal
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn is_expired(&self) -> bool {
        self.expired
    }

    pub fn refresh_last_request(&mut self) {
        self.last_request = Utc::now();
    }

    pub fn refresh_last_request_at(&mut self, last_request: DateTime<Utc>) {
        self.last_request = last_request;
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::SessionInformation;

    #[test]
    fn session_information_tracks_expiration_and_last_request() {
        let start = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let later = Utc.timestamp_opt(1_700_000_100, 0).unwrap();
        let mut info = SessionInformation::new("alice", "s1", start);

        assert_eq!(info.principal(), "alice");
        assert_eq!(info.session_id(), "s1");
        assert!(!info.is_expired());

        info.expire_now();
        info.refresh_last_request_at(later);

        assert!(info.is_expired());
        assert_eq!(info.last_request(), later);
    }
}
