use super::session_information::SessionInformation;

pub trait SessionRegistry
where
    Self: Send + Sync,
{
    fn all_principals(&self) -> Vec<String>;

    fn all_sessions(
        &self,
        principal: &str,
        include_expired_sessions: bool,
    ) -> Vec<SessionInformation>;

    fn session_information(&self, session_id: &str) -> Option<SessionInformation>;

    fn refresh_last_request(&self, session_id: &str);

    fn register_new_session(&self, session_id: &str, principal: &str);

    fn remove_session_information(&self, session_id: &str);
}
