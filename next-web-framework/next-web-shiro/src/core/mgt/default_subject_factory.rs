use crate::core::mgt::subject_factory::SubjectFactory;



#[derive(Clone)]
pub struct DefaultSubjectFactory {}

impl SubjectFactory for DefaultSubjectFactory {}

impl Default for DefaultSubjectFactory {
    fn default() -> Self {
        Self {  }
    }
}