use crate::core::{authc::authenticator::Authenticator, realm::Realm};

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
        todo!()
    }
}
