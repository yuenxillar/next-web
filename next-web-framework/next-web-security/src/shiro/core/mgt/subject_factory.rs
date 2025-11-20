use next_web_core::async_trait;

use crate::core::subject::{subject_context::SubjectContext, Subject};

#[async_trait]
pub trait SubjectFactory
where
    Self: Send + Sync,
{
    async fn create_subject(&self, context: &dyn SubjectContext) -> Box<dyn Subject>;
}
