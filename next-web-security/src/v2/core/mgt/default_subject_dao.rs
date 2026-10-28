use std::sync::Arc;

use next_web_core::async_trait;

#[cfg(feature = "web")]
use crate::web::subject::web_subject::WebSubject;
use crate::core::{
    mgt::{session_storage_evaluator::SessionStorageEvaluator, subject_dao::SubjectDAO},
    session::SessionValue,
    subject::{
        principal_collection::PrincipalCollection,
        support::default_subject_context::DefaultSubjectContext, Subject,
    },
};

#[cfg(feature = "web")]
use crate::web::mgt::default_web_session_storage_evaluator::DefaultWebSessionStorageEvaluator;

#[cfg(not(feature = "web"))]
use crate::core::mgt::default_session_storage_evaluator::DefaultSessionStorageEvaluator;

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

    async fn save_to_session(&self, subject: &dyn Subject) {
        // performs merge logic, only updating the Subject's session if it does not match the current state:
        self.merge_principals(subject).await;
        self.merge_authentication_state(subject).await;
    }

    async fn merge_principals(&self, subject: &dyn Subject) {
        // The identity that belongs in the session: the identity the subject
        // serves, or — for a subject acting as another identity — the identity
        // it acts for, which is what the session must keep.
        let current_principals: Option<Arc<dyn PrincipalCollection>> =
            if subject.is_run_as().await {
                subject.get_previous_principals().await
            } else {
                subject.get_principals().await.cloned()
            };

        // A subject without a session has nowhere to keep its identity. The
        // security manager opens the session of a subject that has an identity
        // before it saves it, so this is a subject that has none.
        let Some(session) = subject.get_session() else {
            return;
        };

        let existing: Option<Arc<dyn PrincipalCollection>> = session
            .get_attribute(DefaultSubjectContext::PRINCIPALS_SESSION_KEY)
            .await
            .and_then(|value| value.to_value::<Arc<dyn PrincipalCollection>>().cloned());

        match current_principals {
            Some(current) if !current.is_empty() => {
                // The session keeps the identity the subject serves; it is
                // written only when it differs from the one already there.
                let unchanged = existing
                    .as_ref()
                    .map(|existing| existing.id() == current.id())
                    .unwrap_or_default();

                if !unchanged {
                    session
                        .set_attribute(
                            DefaultSubjectContext::PRINCIPALS_SESSION_KEY,
                            SessionValue::Object(Box::new(current)),
                        )
                        .await
                        .ok();
                }
            }
            _ => {
                // The subject serves no identity any more, so the session must
                // not claim that it does.
                if existing.is_some() {
                    session
                        .remove_attribute(DefaultSubjectContext::PRINCIPALS_SESSION_KEY)
                        .await
                        .ok();
                }
            }
        };
    }

    async fn merge_authentication_state(&self, subject: &dyn Subject) {
        let session = subject.get_session();

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
                        // otherwise the authc state matches - no need to update the session
                    }
                    // otherwise the session already records the authentication
                } else if existing_authc {
                    // the subject is no longer authenticated, so the session must not
                    // claim that it is
                    session
                        .remove_attribute(DefaultSubjectContext::AUTHENTICATED_SESSION_KEY)
                        .await
                        .ok();
                }
            }
            None => {
                // No session and no authenticated subject: nothing to save.
            }
        };
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
                subject.get_principal().await.map(|principal| principal.to_string()).unwrap_or_default()
            )
        }
    }

    async fn delete(&self, subject: &dyn WebSubject) {
        remove_stored_state(subject).await;
    }
}

#[cfg(not(feature = "web"))]
#[async_trait]
impl SubjectDAO for DefaultSubjectDAO {
    async fn save(&self, subject: &dyn Subject) {
        if self.is_session_storage_enabled(subject).await {
            self.save_to_session(subject).await;
        } else {
            tracing::trace!(
                "Session storage of subject state for Subject [{}] has been disabled: identity and
                authentication state are expected to be initialized on every request or invocation.",
                subject.get_principal().await.map(|principal| principal.to_string()).unwrap_or_default()
            )
        }
    }

    async fn delete(&self, subject: &dyn Subject) {
        remove_stored_state(subject).await;
    }
}

/// Removes the state of a subject from its session, the way a logout does.
async fn remove_stored_state(subject: &dyn Subject) {
    if let Some(session) = subject.get_session() {
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

impl Default for DefaultSubjectDAO {
    fn default() -> Self {
        #[cfg(feature = "web")]
        let session_storage_evaluator: Box<dyn SessionStorageEvaluator> =
            Box::new(DefaultWebSessionStorageEvaluator::default());

        #[cfg(not(feature = "web"))]
        let session_storage_evaluator: Box<dyn SessionStorageEvaluator> =
            Box::new(DefaultSessionStorageEvaluator::default());

        Self {
            session_storage_evaluator,
        }
    }
}
