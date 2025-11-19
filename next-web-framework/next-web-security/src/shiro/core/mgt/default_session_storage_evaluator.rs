use next_web_core::async_trait;

use crate::core::mgt::session_storage_evaluator::SessionStorageEvaluator;

#[derive(Clone)]
pub struct DefaultSessionStorageEvaluator {
    session_storage_enabled: bool,
}

impl DefaultSessionStorageEvaluator {
    pub fn is_session_storage_enabled(&self) -> bool {
        self.session_storage_enabled
    }

    pub fn set_session_storage_enabled(&mut self, session_storage_enabled: bool) {
        self.session_storage_enabled = session_storage_enabled;
    }
}

#[async_trait]
impl SessionStorageEvaluator for DefaultSessionStorageEvaluator {
    async fn is_session_storage_enabled(
        &self,
        subject: &dyn crate::core::subject::Subject,
    ) -> bool {
        subject.get_session().is_some() || self.is_session_storage_enabled()
    }
}

impl Default for DefaultSessionStorageEvaluator {
    fn default() -> Self {
        Self {
            session_storage_enabled: true,
        }
    }
}
