use crate::core::subject::principal_collection::PrincipalCollection;


pub trait LogoutAware {
    
    fn on_logout(&mut self, principals: &dyn PrincipalCollection);
}