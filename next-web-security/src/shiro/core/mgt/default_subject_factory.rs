use next_web_core::async_trait;

use crate::{
    core::{mgt::subject_factory::SubjectFactory, subject::Subject},
    web::subject::web_subject_context::WebSubjectContext,
};

#[derive(Clone)]
pub struct DefaultSubjectFactory {}

#[cfg(not(feature = "web"))]
#[async_trait]
impl SubjectFactory for DefaultSubjectFactory {
    async fn create_subject(&self, context: &dyn WebSubjectContext) -> Box<dyn Subject> {
        todo!()
    }
}

impl Default for DefaultSubjectFactory {
    fn default() -> Self {
        Self {}
    }
}
