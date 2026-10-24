use std::sync::Arc;

use crate::core::session::SessionId;
use next_web_core::async_trait;

use crate::core::session::{mgt::eis::session_dao::SessionDAO, Session};

#[derive(Clone)]
pub struct MemorySessionDAO {}

impl MemorySessionDAO {
    fn generate_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[async_trait]
impl SessionDAO for MemorySessionDAO {
    async fn create(&self, session: Arc<dyn Session>) {}

    async fn read(&self, session_id: &SessionId) -> Option<&Arc<dyn Session>> {
        None
    }

    async fn update(&self, session: Arc<dyn Session>) -> Result<(), &'static str> {
        Ok(())
    }

    async fn delete(&self, session: &dyn Session) {}

    async fn get_active_sessions(&self) -> Vec<&Arc<dyn Session>> {
        vec![]
    }
}

impl Default for MemorySessionDAO {
    fn default() -> Self {
        Self {}
    }
}
