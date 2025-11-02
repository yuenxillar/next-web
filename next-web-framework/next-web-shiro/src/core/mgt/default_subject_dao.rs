use std::sync::Arc;

use crate::{core::mgt::{
    session_storage_evaluator::SessionStorageEvaluator, subject_dao::SubjectDAO,
}, web::mgt::default_web_session_storage_evaluator::DefaultWebSessionStorageEvaluator};

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

    pub fn get_session_storage_evaluator(&self) ->  &dyn SessionStorageEvaluator {
        self.session_storage_evaluator.as_ref()
    }

    pub fn get_mut_session_storage_evaluator(&mut self) ->  &mut dyn SessionStorageEvaluator {
        self.session_storage_evaluator.as_mut()
    }
}

impl SubjectDAO for DefaultSubjectDAO {
    fn save(&self, subject: &dyn crate::core::subject::Subject) {
        todo!()
    }

    fn delete(&self, subject: &dyn crate::core::subject::Subject) {
        todo!()
    }
}

impl Default for DefaultSubjectDAO {
    fn default() -> Self {
        Self {
            session_storage_evaluator: Box::new(DefaultWebSessionStorageEvaluator::default())
        }
    }
}
