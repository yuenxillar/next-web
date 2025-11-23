#[cfg(not(feature = "web"))]
use crate::core::subject::Subject;
#[cfg(feature = "web")]
use crate::web::subject::web_subject::WebSubject;
#[cfg(feature = "web")]
use next_web_core::async_trait;

#[cfg(not(feature = "web"))]
pub trait SubjectDAO
where
    Self: Send,
{
    fn save(&self, subject: &dyn Subject);

    fn delete(&self, subject: &dyn Subject);
}

#[cfg(feature = "web")]
#[async_trait]
pub trait SubjectDAO
where
    Self: Send + Sync,
{
    async fn save(&self, subject: &dyn WebSubject);

    async fn delete(&self, subject: &dyn WebSubject);
}
