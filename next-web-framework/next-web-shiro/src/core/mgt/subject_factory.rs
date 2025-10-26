use crate::core::subject::{subject_context::SubjectContext, Subject};

pub trait SubjectFactory
where 
Self: Send + Sync
{
    fn create_subject(&self, context: &dyn SubjectContext) -> Box<dyn Subject>;
}