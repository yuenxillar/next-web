use next_web_core::async_trait;

use crate::{
    core::{
        mgt::{default_subject_factory::DefaultSubjectFactory, subject_factory::SubjectFactory},
        subject::subject_context::SubjectContext,
    },
    web::subject::support::web_delegating_subject::WebDelegatingSubject,
};

#[derive(Clone)]
pub struct DefaultWebSubjectFactory {
    default_subject_factory: DefaultSubjectFactory,
}

#[async_trait]
impl SubjectFactory for DefaultWebSubjectFactory {
    async fn create_subject(
        &self,
        context: &dyn SubjectContext,
    ) -> Box<dyn crate::core::subject::Subject> {
        let principals = context.resolve_principals().await.cloned();
        let authenticated = context.resolve_authenticated().await;
        let host = context.resolve_host().await;
        let session = context.resolve_session().cloned();

        let session_enabled = context.is_session_creation_enabled();
        let security_manager = context.resolve_security_manager().await.unwrap().clone();
        Box::new(WebDelegatingSubject::new(
            principals,
            authenticated,
            host,
            session,
            session_enabled,
            security_manager,
        ))
    }
}

impl Default for DefaultWebSubjectFactory {
    fn default() -> Self {
        Self {
            default_subject_factory: Default::default(),
        }
    }
}
