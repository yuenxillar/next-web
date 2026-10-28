#[cfg(not(feature = "web"))]
use std::sync::Arc;

#[cfg(not(feature = "web"))]
use next_web_core::async_trait;

#[cfg(not(feature = "web"))]
use crate::core::{
    mgt::{security_manager::SecurityManager, subject_factory::SubjectFactory},
    subject::{
        subject_context::SubjectContext, support::delegating_subject::DelegatingSubject, Subject,
    },
};

/// The factory of the subject of an application that has no web integration.
///
/// The web tree builds a `WebDelegatingSubject`, which also carries the request
/// and the response of its call; without the web integration the subject is the
/// plain `DelegatingSubject`.
#[derive(Clone)]
pub struct DefaultSubjectFactory {}

#[cfg(not(feature = "web"))]
#[async_trait]
impl SubjectFactory for DefaultSubjectFactory {
    async fn create_subject(&self, context: &dyn SubjectContext) -> Box<dyn Subject> {
        let principals = context.resolve_principals().await;
        let authenticated = context.resolve_authenticated().await;
        let host = context.resolve_host().await;
        let session = context.resolve_session().cloned();
        let session_enabled = context.is_session_creation_enabled();
        // The context names the manager the subject works through; a context
        // that was built without one cannot answer the authorization questions
        // of the subject, so it is an error to build the subject from it.
        let security_manager: Arc<dyn SecurityManager> = context
            .resolve_security_manager()
            .await
            .map(Clone::clone)
            .unwrap();

        Box::new(DelegatingSubject::new(
            principals,
            authenticated,
            host,
            session,
            session_enabled,
            security_manager,
        ))
    }
}

impl Default for DefaultSubjectFactory {
    fn default() -> Self {
        Self {}
    }
}
