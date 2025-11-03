use crate::core::mgt::{default_subject_factory::DefaultSubjectFactory, subject_factory::SubjectFactory};

#[derive(Clone)]
pub struct DefaultWebSubjectFactory {
    default_subject_factory: DefaultSubjectFactory,
}

impl SubjectFactory for DefaultWebSubjectFactory {
    fn create_subject(
        &self,
        context: &dyn crate::core::subject::subject_context::SubjectContext,
    ) -> Box<dyn crate::core::subject::Subject> {
        todo!()
    }
}

impl Default for DefaultWebSubjectFactory {
    fn default() -> Self {
        Self {
            default_subject_factory: Default::default(),
        }
    }
}
