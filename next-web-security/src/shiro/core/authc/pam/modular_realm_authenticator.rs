use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authenticator::Authenticator,
        logout_aware::LogoutAware,
    },
    realm::Realm,
};
use tracing::warn;

#[derive(Clone)]
pub struct ModularRealmAuthenticator {
    // TODO: implement this
}

impl ModularRealmAuthenticator {
    pub fn set_realms<T: Realm>(&mut self, realms: Vec<T>) {}
}
impl Default for ModularRealmAuthenticator {
    fn default() -> Self {
        Self {}
    }
}

impl Authenticator for ModularRealmAuthenticator {
    fn authenticate(
        &self,
        authentication_token: &dyn crate::core::authc::authentication_token::AuthenticationToken,
    ) -> Result<
        Box<dyn crate::core::authc::authentication_info::AuthenticationInfo>,
        crate::core::authc::authentication_error::AuthenticationError,
    > {
        Err(AuthenticationError::NotImplemented(
            "ModularRealmAuthenticator::authenticate is not implemented".to_string(),
        ))
    }
}

impl LogoutAware for ModularRealmAuthenticator {
    fn on_logout(
        &self,
        principals: &dyn crate::core::subject::principal_collection::PrincipalCollection,
    ) {
        let _ = principals;
        warn!("ModularRealmAuthenticator::on_logout is not implemented");
    }
}
