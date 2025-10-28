use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::core::{
    mgt::security_manager::SecurityManager,
    subject::{Subject, support::default_subject_context::DefaultSubjectContext},
    util::object::Object,
};

#[derive(Clone)]
pub struct WebSecurityContext {
    pub(crate) resources: Arc<RwLock<HashMap<String, Arc<dyn Subject>>>>,
    pub(crate) security_manager: Arc<dyn SecurityManager>,
}

impl WebSecurityContext {
    pub async fn get_subject(&self, id: &str) -> Arc<dyn Subject> {
        if let Some(subject) = self.resources.read().await.get(id)
        {
            return subject.to_owned();
        }
        
        self.security_manager
            .create_subject(Arc::new(DefaultSubjectContext::default()))
    }
}
