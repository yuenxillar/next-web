use crate::core::mgt::subject_dao::SubjectDAO;


#[derive(Clone)]
pub struct DefaultSubjectDAO {}


impl SubjectDAO for DefaultSubjectDAO {
}


impl Default for DefaultSubjectDAO {
    fn default() -> Self {
        Self {  }
    }
}