use next_web_core::async_trait;

use crate::core::{
    mgt::subject_factory::SubjectFactory,
    subject::{Subject, subject_context::SubjectContext},
};

#[derive(Clone)]
pub struct DefaultSubjectFactory {}

#[async_trait]
impl SubjectFactory for DefaultSubjectFactory {
    async fn create_subject(&self, context: &dyn SubjectContext) -> Box<dyn Subject> {
        todo!()
    }
}

impl Default for DefaultSubjectFactory {
    fn default() -> Self {
        Self {}
    }
}
