use next_web_core::async_trait;

#[cfg(not(feature = "web"))]
use crate::core::subject::Subject;
#[cfg(feature = "web")]
use crate::web::subject::web_subject::WebSubject;

/// Stores the state of a subject where it outlives the call.
///
/// The tree without the web integration keeps that state in the session of the
/// subject; the web integration does the same, through the subject of the web
/// call. Both need the session, so both variants of the trait are asynchronous.
#[cfg(not(feature = "web"))]
#[async_trait]
pub trait SubjectDAO
where
    Self: Send + Sync,
{
    async fn save(&self, subject: &dyn Subject);

    async fn delete(&self, subject: &dyn Subject);
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
