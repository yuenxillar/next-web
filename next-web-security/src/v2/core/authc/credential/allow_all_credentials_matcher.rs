use crate::core::authc::{
    authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
    credential::credentials_matcher::CredentialsMatcher,
};

/// A matcher that accepts every credential.
///
/// Apache Shiro has the same matcher for the realms that have already verified
/// the credential — a token minted after a check that cannot be repeated, for
/// instance — and for an application that decides somewhere else. It is never
/// the default: a realm that accepts everything lets any token in.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllCredentialsMatcher;

impl AllowAllCredentialsMatcher {
    /// Creates the matcher.
    pub fn new() -> Self {
        Self
    }
}

impl CredentialsMatcher for AllowAllCredentialsMatcher {
    fn do_credentials_match(
        &self,
        _token: &dyn AuthenticationToken,
        _info: &dyn AuthenticationInfo,
    ) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::AllowAllCredentialsMatcher;
    use crate::core::{
        authc::{
            authentication_info::AuthenticationInfo,
            credential::credentials_matcher::CredentialsMatcher,
            simple_authentication_info::SimpleAuthenticationInfo,
            username_password_token::UsernamePasswordToken,
        },
        subject::simple_principal_collection::SimplePrincipalCollection,
        util::object::Object,
    };

    #[test]
    fn every_credential_is_accepted() {
        let principals = Arc::new(SimplePrincipalCollection::new(
            [Object::Str("root".to_string())],
            "realm",
        ));
        let info = SimpleAuthenticationInfo::new(principals, None);
        let token =
            UsernamePasswordToken::new("root".to_string(), "wrong".to_string(), false, None);

        assert!(AllowAllCredentialsMatcher::new().do_credentials_match(&token, &info));
        assert!(matches!(
            info.get_credentials(),
            None
        ));
    }
}
