use crate::core::authc::credential::credentials_matcher::CredentialsMatcher;

pub struct SimpleCredentialsMatcher {}

impl CredentialsMatcher for SimpleCredentialsMatcher {
    fn do_credentials_match(
        &self,
        token: &dyn crate::core::authc::authentication_token::AuthenticationToken,
        info: &dyn crate::core::authc::authentication_info::AuthenticationInfo,
    ) -> bool {
        token.get_credentials().map(|value| value.to_string())
            == info.get_credentials().map(ToString::to_string)
    }
}

impl Default for SimpleCredentialsMatcher {
    fn default() -> Self {
        Self {}
    }
}
