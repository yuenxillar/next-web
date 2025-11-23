use std::sync::Arc;

use next_web_core::async_trait;

#[cfg(feature = "web")]
use crate::web::subject::web_subject::WebSubject;
use crate::{
    core::{
        mgt::{session_storage_evaluator::SessionStorageEvaluator, subject_dao::SubjectDAO},
        session::SessionValue,
        subject::{
            principal_collection::PrincipalCollection,
            support::default_subject_context::DefaultSubjectContext, Subject,
        },
    },
    web::mgt::default_web_session_storage_evaluator::DefaultWebSessionStorageEvaluator,
};

#[derive(Clone)]
pub struct DefaultSubjectDAO {
    session_storage_evaluator: Box<dyn SessionStorageEvaluator>,
}

impl DefaultSubjectDAO {
    pub fn set_session_storage_evaluator<T: SessionStorageEvaluator + 'static>(
        &mut self,
        session_storage_evaluator: T,
    ) {
        self.session_storage_evaluator = Box::new(session_storage_evaluator);
    }

    pub fn get_session_storage_evaluator(&self) -> &dyn SessionStorageEvaluator {
        self.session_storage_evaluator.as_ref()
    }

    pub fn get_mut_session_storage_evaluator(&mut self) -> &mut dyn SessionStorageEvaluator {
        self.session_storage_evaluator.as_mut()
    }

    async fn is_session_storage_enabled(&self, subject: &dyn Subject) -> bool {
        self.get_session_storage_evaluator()
            .is_session_storage_enabled(subject)
            .await
    }

    async fn save_to_session(&self, subject: &dyn WebSubject) {
        // performs merge logic, only updating the Subject's session if it does not match the current state:
        self.merge_principals(subject).await;
        self.merge_authentication_state(subject).await;
    }

    fn is_empty(&self, pc: &dyn PrincipalCollection) -> bool {
        pc.is_empty()
    }

    async fn merge_principals(&self, subject: &dyn WebSubject) {
        // merge PrincipalCollection state:

        let mut current_principals: Option<&Arc<dyn PrincipalCollection>> = None;
        if subject.is_run_as().await {
            current_principals = subject.get_principals().await;
        }

        let session = subject.get_session();
        match session {
            Some(session) => {
                let value: Option<SessionValue> = session
                    .get_attribute(DefaultSubjectContext::PRINCIPALS_SESSION_KEY)
                    .await;
                if let Some(value) = value {
                    if let Some(pc) = value.as_object::<Arc<dyn PrincipalCollection>>() {
                        if current_principals.map(|a| a.is_empty()).unwrap_or(true) {
                            if !pc.is_empty() {
                                session
                                    .remove_attribute(DefaultSubjectContext::PRINCIPALS_SESSION_KEY)
                                    .await
                                    .ok();
                            }
                            // otherwise both are null or empty - no need to update the session
                        } else {
                            if let Some(current_principals) = current_principals {
                                if !(current_principals.id() == pc.id()) {
                                    session
                                        .set_attribute(
                                            DefaultSubjectContext::PRINCIPALS_SESSION_KEY,
                                            SessionValue::Object(Box::new(
                                                current_principals.clone(),
                                            )),
                                        )
                                        .await
                                        .ok();
                                }
                            }
                        }
                        // otherwise they're the same - no need to update the session
                    }
                }
            }
            None => {
                if current_principals
                    .map(|a| !a.is_empty())
                    .unwrap_or_default()
                {
                    if let Some(session) = subject.get_session() {
                        session
                            .set_attribute(
                                DefaultSubjectContext::PRINCIPALS_SESSION_KEY,
                                SessionValue::Object(Box::new(session.clone())),
                            )
                            .await
                            .ok();
                    }
                }
            }
        };
    }

    async fn merge_authentication_state(&self, subject: &dyn WebSubject) {
        let mut session = subject.get_session();

        match session {
            Some(session) => {
                let existing_authc: bool = session
                    .get_attribute(DefaultSubjectContext::AUTHENTICATED_SESSION_KEY)
                    .await
                    .map(|val| val.as_boolean().unwrap_or_default())
                    .unwrap_or_default();

                if subject.is_authenticated().await {
                    if !existing_authc {
                        session
                            .set_attribute(
                                DefaultSubjectContext::AUTHENTICATED_SESSION_KEY,
                                SessionValue::Boolean(true),
                            )
                            .await
                            .ok();
                        //otherwise authc state matches - no need to update the session
                    } else {
                        //existing doesn't match the current state - remove it:
                        session
                            .remove_attribute(DefaultSubjectContext::AUTHENTICATED_SESSION_KEY)
                            .await
                            .ok();

                        //otherwise not in the session and not authenticated - no need to update the session
                    }
                }
            }
            None => {
                if subject.is_authenticated().await {
                    session = subject.get_session();
                    if let Some(session) = session.as_ref() {
                        session
                            .set_attribute(
                                DefaultSubjectContext::AUTHENTICATED_SESSION_KEY,
                                SessionValue::Boolean(true),
                            )
                            .await
                            .ok();
                    }

                    //otherwise no session and not authenticated - nothing to save
                }
            }
        };

        todo!()
    }
}

#[cfg(feature = "web")]
#[async_trait]
impl SubjectDAO for DefaultSubjectDAO {
    async fn save(&self, subject: &dyn WebSubject) {
        if self.is_session_storage_enabled(subject).await {
            self.save_to_session(subject).await;
        } else {
            tracing::trace!(
                "Session storage of subject state for Subject [{}] has been disabled: identity and 
                authentication state are expected to be initialized on every request or invocation.",
                "todo!()"
            )
        }
    }

    async fn delete(&self, subject: &dyn WebSubject) {
        if let Some(session) = subject.get_session() {
            use crate::core::subject::support::default_subject_context::DefaultSubjectContext;

            session
                .remove_attribute(DefaultSubjectContext::AUTHENTICATED_SESSION_KEY)
                .await
                .ok();
            session
                .remove_attribute(DefaultSubjectContext::PRINCIPALS_SESSION_KEY)
                .await
                .ok();
        }
    }
}

impl Default for DefaultSubjectDAO {
    fn default() -> Self {
        Self {
            session_storage_evaluator: Box::new(DefaultWebSessionStorageEvaluator::default()),
        }
    }
}
