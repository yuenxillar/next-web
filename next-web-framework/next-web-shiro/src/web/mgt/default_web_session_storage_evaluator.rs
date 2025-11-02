use std::any::Any;

use crate::{
    core::{
        mgt::{
            default_session_storage_evaluator::DefaultSessionStorageEvaluator,
            session_storage_evaluator::SessionStorageEvaluator,
        },
        session::mgt::session_manager::SessionManager,
    },
    web::{
        session::mgt::default_web_session_manager::DefaultWebSessionManager,
        subject::support::web_delegating_subject::WebDelegatingSubject,
    },
};

#[derive(Clone)]
pub struct DefaultWebSessionStorageEvaluator<S = DefaultWebSessionManager> {
    session_manager: Option<S>,
    default_session_storage_evaluator: DefaultSessionStorageEvaluator,
}

impl<S> DefaultWebSessionStorageEvaluator<S>
where
    S: SessionManager,
{
    pub fn set_session_manager(&mut self, session_manager: S) {
        self.session_manager = Some(session_manager);
    }
}

impl<S> SessionStorageEvaluator for DefaultWebSessionStorageEvaluator<S>
where
    S: SessionManager + 'static,
    S: Clone
{
    fn is_session_storage_enabled(&self, subject: &dyn crate::core::subject::Subject) -> bool {
        if subject.get_session().is_some() {
            return true;
        }

        if !self
            .default_session_storage_evaluator
            .is_session_storage_enabled()
        {
            return false;
        }

        if !(subject as &dyn Any)
            .downcast_ref::<WebDelegatingSubject>()
            .is_none()
            && (self.session_manager.is_some())
        {
            return false;
        }

        true
    }
}

impl Default for DefaultWebSessionStorageEvaluator {
    fn default() -> Self {
        Self {
            default_session_storage_evaluator: Default::default(),
            session_manager: Default::default(),
        }
    }
}
