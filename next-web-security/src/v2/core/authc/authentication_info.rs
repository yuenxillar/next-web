use std::{any::Any, fmt::Display, sync::Arc};

use next_web_core::DynClone;

use crate::core::{subject::principal_collection::PrincipalCollection, util::object::Object};

pub trait AuthenticationInfo
where
    Self: Send + Sync,
    Self: Display + Any,
    Self: DynClone,
{
    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>>;

    fn get_credentials(&self) -> Option<&Object>;

    /// Returns the salt the stored credentials were hashed with, when they were.
    ///
    /// Apache Shiro models this as the `SaltedAuthenticationInfo` sub-interface;
    /// the port keeps it on the trait so a matcher can ask any info for its salt
    /// without knowing which realm produced it.
    fn get_credentials_salt(&self) -> Option<&[u8]> {
        None
    }
}

next_web_core::clone_trait_object!(AuthenticationInfo);
