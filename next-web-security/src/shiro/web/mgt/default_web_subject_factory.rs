use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    core::mgt::subject_factory::SubjectFactory,
    web::{
        mgt::web_security_manager::WebSecurityManager,
        subject::{
            support::web_delegating_subject::WebDelegatingSubject, web_subject::WebSubject,
            web_subject_context::WebSubjectContext,
        },
    },
};

#[derive(Clone)]
pub struct DefaultWebSubjectFactory;

#[async_trait]
impl SubjectFactory for DefaultWebSubjectFactory {
    async fn create_subject(&self, context: &dyn WebSubjectContext) -> Box<dyn WebSubject> {
        let principals = context.resolve_principals().await.cloned();
        let authenticated = context.resolve_authenticated().await;
        let host = context.resolve_host().await;
        let session = context.resolve_session().cloned();

        let session_enabled = context.is_session_creation_enabled();
        let security_manager: Arc<dyn WebSecurityManager> = context
            .resolve_security_manager()
            .await
            .map(|val| val.clone())
            .unwrap();

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
        Self {}
    }
}
