use next_web_core::async_trait;

#[cfg(not(feature = "web"))]
use crate::core::subject::Subject;

#[cfg(not(feature = "web"))]
use crate::core::subject::subject_context::SubjectContext;
#[cfg(feature = "web")]
use crate::web::subject::{web_subject::WebSubject, web_subject_context::WebSubjectContext};

#[async_trait]
pub trait SubjectFactory
where
    Self: Send,
{
    #[cfg(not(feature = "web"))]
    async fn create_subject(&self, context: &dyn SubjectContext) -> Box<dyn Subject>;

    #[cfg(feature = "web")]
    async fn create_subject(&self, context: &dyn WebSubjectContext) -> Box<dyn WebSubject>;
}
