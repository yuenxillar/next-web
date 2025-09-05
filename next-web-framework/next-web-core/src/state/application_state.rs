use std::{borrow::Cow, sync::Arc};

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
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut Arc<RwLock<ApplicationContext>> {
        &mut self.context
    }

    pub async fn get_single_with_name<T>(&self, name: impl Into<Cow<'static, str>>) -> T
    where
        T: Send + Sync + Clone,
        T: 'static,
    {
        let name = name.into();
        let reader = self.context.read().await;

        match reader.get_single_option_with_name::<T>(name.clone()) {
            Some(instance_with_name) => instance_with_name.clone(),
            None => match reader.get_single_option_with_name::<T>("") {
                Some(instance) => instance.clone(),
                None => {
                    drop(reader);
                    match self
                        .context
                        .write()
                        .await
                        .resolve_option_with_name_async::<T>(name.clone())
                        .await
                    {
                        Some(instance) => instance,
                        _ => panic!("No instance found for {}", name),
                    }
                }
            },
        }
    }
}
