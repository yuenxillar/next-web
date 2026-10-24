use crate::core::subject::principal_collection::PrincipalCollection;

pub trait LogoutAware
where
    Self: Send + Sync,
{
    fn on_logout(&self, principals: &dyn PrincipalCollection);
}
