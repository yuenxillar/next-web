use std::any::Any;

use next_web_core::async_trait;

use super::session_information::SessionInformation;

#[async_trait]
pub trait SessionRegistry
where
    Self: Send + Sync,
{
    async fn all_principals(&self) -> Vec<String>;

    async fn all_sessions(
        &self,
        principal: &dyn Any,
        include_expired_sessions: bool,
    ) -> Vec<SessionInformation>;

    async fn session_information<'a>(&'a self, session_id: &str) -> Option<&'a SessionInformation>;

    async fn refresh_last_request(&self, session_id: &str);

    async fn register_new_session(&self, session_id: &str, principal: &dyn Any);

    async fn remove_session_information(&self, session_id: &str);
}
