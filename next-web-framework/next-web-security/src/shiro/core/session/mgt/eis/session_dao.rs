use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::session::{Session, SessionId};

#[async_trait]
pub trait SessionDAO
where

    Self: Send + Sync,
{
    async fn create(&mut self, session: Arc<dyn Session>);

    async fn read(&self, session_id: &SessionId) -> Option<&Arc<dyn Session>>;

    async fn update(&mut self, session: Arc<dyn Session>) -> Result<(), &'static str>;

    async fn delete(&mut self, session: Arc<dyn Session>);

    async fn get_active_sessions(&self) -> Vec<&Arc<dyn Session>>;
}
