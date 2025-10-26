use crate::core::mgt::subject_dao::SubjectDAO;


#[derive(Clone)]
pub struct DefaultSubjectDAO {}


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
        Self {  }
    }
}