use crate::core::subject::Subject;



pub trait SubjectDAO
where 
Self: Send + Sync
{

    fn save(&self, subject: &dyn Subject);

    fn delete(&self, subject: &dyn Subject);
}