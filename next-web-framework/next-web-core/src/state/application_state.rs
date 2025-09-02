use std::sync::Arc;

use tokio::sync::RwLock;

use crate::ApplicationContext;

#[derive(Clone)]
pub struct ApplicationState {
    pub(crate) context: Arc<RwLock<ApplicationContext>>,
}

impl ApplicationState {
    pub fn from_context(application_context: ApplicationContext) -> Self {
        let context: Arc<RwLock<ApplicationContext>> = Arc::new(RwLock::new(application_context));
        Self { context }
    }

    pub fn context(&self) -> &Arc<RwLock<ApplicationContext>> {
        & self.context
    }

    pub fn context_mut(&mut self) -> &mut Arc<RwLock<ApplicationContext>> {
        &mut self.context
    }
}