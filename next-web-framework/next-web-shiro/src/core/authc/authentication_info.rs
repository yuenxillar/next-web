use crate::core::{object::Object, subject::principal_collection::PrincipalCollection};

pub trait AuthenticationInfo
where
    Self: Send,
{

    fn get_principals(&self) -> Option<&dyn PrincipalCollection>;


    fn get_credentials(&self) -> Option<& Object>;
}
