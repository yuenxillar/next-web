use next_web_core::DynClone;

use crate::core::authz::permission::Permission;

pub trait AuthorizationInfo
where 
Self: Send + Sync,
Self: DynClone 
{
    fn get_roles(&self) -> Vec<String>;

    fn get_permissions(&self) -> Vec<String>;

    fn get_dyn_permissions(&self) ->  Vec<Box<dyn Permission>>;
}


next_web_core::clone_trait_object!(AuthorizationInfo);