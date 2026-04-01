use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{error::BoxError, traits::schedule::scheduled_job_handler::ScheduledJobHandler};

#[derive(Clone, Default)]
pub struct ScheduledJobRegistry {
    handlers: Arc<RwLock<HashMap<String, Arc<dyn ScheduledJobHandler>>>>,
}

impl ScheduledJobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, handler: Arc<dyn ScheduledJobHandler>) -> Result<(), BoxError> {
        let task_key = handler.id().to_string();
        let mut handlers = self
            .handlers
            .write()
            .map_err(|error| -> BoxError { Box::new(std::io::Error::other(error.to_string())) })?;

        if handlers.contains_key(&task_key) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("scheduled job handler already registered: {}", task_key),
            )));
        }

        handlers.insert(task_key, handler);
        Ok(())
    }

    pub fn get(&self, task_key: &str) -> Option<Arc<dyn ScheduledJobHandler>> {
        self.handlers
            .read()
            .ok()
            .and_then(|handlers| handlers.get(task_key).cloned())
    }

    pub fn task_keys(&self) -> Vec<String> {
        self.handlers
            .read()
            .map(|handlers| handlers.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{Value, json};

    use crate::{
        error::BoxError, scheduler::context::JobExecutionContext,
        traits::schedule::scheduled_job_handler::ScheduledJobHandler,
    };

    use super::ScheduledJobRegistry;

    struct TestHandler {
        task_key: &'static str,
    }

    #[crate::async_trait]
    impl ScheduledJobHandler for TestHandler {
        fn id(&self) -> &str {
            self.task_key
        }

        async fn execute(
            &self,
            _context: JobExecutionContext,
            _payload: Option<Value>,
        ) -> Result<(), BoxError> {
            Ok(())
        }
    }

    #[test]
    fn register_and_get_handler() {
        let registry = ScheduledJobRegistry::new();
        registry
            .register(Arc::new(TestHandler {
                task_key: "handler.a",
            }))
            .unwrap();

        assert!(registry.get("handler.a").is_some());
        assert_eq!(registry.task_keys(), vec!["handler.a".to_string()]);
    }

    #[test]
    fn duplicate_task_key_is_rejected() {
        let registry = ScheduledJobRegistry::new();
        registry
            .register(Arc::new(TestHandler {
                task_key: "handler.dup",
            }))
            .unwrap();

        let error = registry
            .register(Arc::new(TestHandler {
                task_key: "handler.dup",
            }))
            .unwrap_err();

        assert!(error.to_string().contains("already registered"));
    }

    #[tokio::test]
    async fn handler_supports_none_and_some_payload() {
        struct PayloadHandler;

        #[crate::async_trait]
        impl ScheduledJobHandler for PayloadHandler {
            fn id(&self) -> &'static str {
                "handler.payload"
            }

            async fn execute(
                &self,
                _context: JobExecutionContext,
                payload: Option<Value>,
            ) -> Result<(), BoxError> {
                match payload {
                    Some(value) => assert_eq!(value, json!({"ok": true})),
                    None => {}
                }
                Ok(())
            }
        }

        let handler = PayloadHandler;
        handler
            .execute(JobExecutionContext::default(), None)
            .await
            .unwrap();
        handler
            .execute(JobExecutionContext::default(), Some(json!({"ok": true})))
            .await
            .unwrap();
    }
}
