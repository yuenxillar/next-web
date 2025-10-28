use std::sync::Arc;

use crate::core::mgt::subject_factory::SubjectFactory;



#[derive(Clone)]
pub struct DefaultSubjectFactory {}

impl SubjectFactory for DefaultSubjectFactory {
    fn create_subject(&self, context: &dyn crate::core::subject::subject_context::SubjectContext) -> Arc<dyn crate::core::subject::Subject> {
        todo!()
    }
}

impl Default for DefaultSubjectFactory {
    fn default() -> Self {
        Self {  }
    }
}