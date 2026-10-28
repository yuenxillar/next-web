//! Shiro's authenticator: one realm for a single-realm configuration, every
//! realm that supports the token for a multi-realm one.

use std::sync::Arc;

use next_web_core::async_trait;
use tracing::{debug, warn};

use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        authenticator::Authenticator, logout_aware::LogoutAware,
        pam::authentication_strategy::{
            AtLeastOneSuccessfulStrategy, AuthenticationStrategy,
        },
        simple_authentication_info::SimpleAuthenticationInfo, AuthenticationError,
    },
    realm::Realm,
    subject::{
        principal_collection::PrincipalCollection,
        simple_principal_collection::SimplePrincipalCollection,
    },
    util::object::Object,
};

/// An authenticator that resolves a token against the realms of the application.
///
/// With one realm it is that realm that decides; with several, every realm that
/// says it supports the token is asked, and the principals of the realms that
/// authenticated it are merged into one authentication. A token no realm
/// supports, and a realm that knows nothing about the token it supports, are both
/// refused with an [`AuthenticationError`].
#[derive(Clone)]
pub struct ModularRealmAuthenticator {
    realms: Vec<Arc<dyn Realm>>,
    authentication_strategy: Arc<dyn AuthenticationStrategy>,
}

impl Default for ModularRealmAuthenticator {
    fn default() -> Self {
        Self {
            realms: Vec::new(),
            authentication_strategy: Arc::new(AtLeastOneSuccessfulStrategy),
        }
    }
}

impl ModularRealmAuthenticator {
    /// Creates an authenticator over the given realms.
    ///
    /// # Arguments
    ///
    /// * `realms` - The realms that authenticate, in the order they are asked.
    pub fn new(realms: Vec<Arc<dyn Realm>>) -> Self {
        Self {
            realms,
            ..Self::default()
        }
    }

    /// Sets what a configuration of several realms requires of them.
    ///
    /// # Arguments
    ///
    /// * `authentication_strategy` - The strategy that decides whether the
    ///   answers of the realms are enough.
    pub fn with_authentication_strategy(
        mut self,
        authentication_strategy: Arc<dyn AuthenticationStrategy>,
    ) -> Self {
        self.authentication_strategy = authentication_strategy;
        self
    }

    /// Returns the strategy of the authenticator.
    pub fn authentication_strategy(&self) -> &dyn AuthenticationStrategy {
        self.authentication_strategy.as_ref()
    }

    /// Replaces the realms of the authenticator.
    ///
    /// # Arguments
    ///
    /// * `realms` - The realms that authenticate, in the order they are asked.
    pub fn set_realms(&mut self, realms: Vec<Arc<dyn Realm>>) {
        self.realms = realms;
    }

    /// Adds a realm.
    ///
    /// # Arguments
    ///
    /// * `realm` - The realm to ask after the ones already held.
    pub fn add_realm(&mut self, realm: Arc<dyn Realm>) {
        self.realms.push(realm);
    }

    /// Returns the realms of the authenticator.
    pub fn get_realms(&self) -> &[Arc<dyn Realm>] {
        &self.realms
    }

    /// Returns whether the authenticator holds no realm at all.
    pub fn is_empty(&self) -> bool {
        self.realms.is_empty()
    }

    /// Authenticates with the only realm of the configuration.
    ///
    /// # Arguments
    ///
    /// * `token` - What the user presented.
    async fn do_single_realm_authentication(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError> {
        let realm = self
            .realms
            .first()
            .expect("a single realm configuration has a realm");

        if !realm.supports(token) {
            return Err(AuthenticationError::Unknown);
        }

        debug!(
            "Authenticating token against the single realm [{}].",
            realm.get_name()
        );

        realm.get_authentication_info(token).await.ok_or_else(|| {
            AuthenticationError::Custom(format!(
                "The realm [{}] returned no authentication info for the token [{token}].",
                realm.get_name()
            ))
        })
    }

    /// Authenticates with every realm that supports the token.
    ///
    /// # Arguments
    ///
    /// * `token` - What the user presented.
    async fn do_multi_realm_authentication(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError> {
        let supporting: Vec<&Arc<dyn Realm>> =
            self.realms.iter().filter(|realm| realm.supports(token)).collect();
        let realm_names: Vec<&str> = supporting.iter().map(|realm| realm.get_name()).collect();
        self.authentication_strategy
            .before_all_attempts(&realm_names);

        let mut infos = Vec::new();
        let mut authenticated_realms = Vec::new();

        for realm in supporting {
            let realm_name = realm.get_name();

            if !self.authentication_strategy.before_attempt(realm_name) {
                continue;
            }

            // A strategy that has what it needs 鈥?one success, for the first
            // successful one 鈥?stops the calls here rather than asking every realm.
            if !self
                .authentication_strategy
                .should_continue(&authenticated_realms)
            {
                debug!("The strategy has what it needs; the remaining realms are not asked.");

                break;
            }

            debug!("Authenticating token against the realm [{}].", realm_name);

            let info = realm.get_authentication_info(token).await;

            self.authentication_strategy
                .after_attempt(realm_name, info.as_deref())?;

            if let Some(info) = info {
                authenticated_realms.push(realm_name.to_string());
                infos.push((realm_name.to_string(), info));
            }
        }

        self.authentication_strategy
            .after_all_attempts(&authenticated_realms)?;

        match infos.len() {
            0 => Err(AuthenticationError::Custom(format!(
                "No realm authenticated the token [{token}]."
            ))),
            1 => Ok(infos.remove(0).1),
            _ => Ok(Box::new(Self::merge_infos(infos))),
        }
    }

    /// Merges the principals of every realm that authenticated the token.
    ///
    /// The credentials of the merged info are the ones of the first realm, which
    /// is what a credentials matcher compares when the realms hold one account.
    ///
    /// # Arguments
    ///
    /// * `infos` - The authentication info of each realm, with its realm name.
    fn merge_infos(infos: Vec<(String, Box<dyn AuthenticationInfo>)>) -> SimpleAuthenticationInfo {
        let mut principals = SimplePrincipalCollection::default();
        let mut credentials = None;

        for (realm_name, info) in infos {
            if credentials.is_none() {
                credentials = info.get_credentials().cloned();
            }

            match info.get_principals() {
                Some(info_principals) => {
                    for principal in Self::principals_of(info_principals.as_ref()) {
                        principals.add(principal, realm_name.clone());
                    }
                }
                None => principals.add(Object::Null, realm_name),
            }
        }

        SimpleAuthenticationInfo::new(Arc::new(principals), credentials)
    }

    /// Returns the principals of an authentication info as values.
    ///
    /// # Arguments
    ///
    /// * `principals` - The principals of the info.
    fn principals_of(principals: &dyn PrincipalCollection) -> Vec<Object> {
        match principals.get_primary_principal() {
            Some(primary) => vec![primary.clone()],
            None => Vec::new(),
        }
    }
}

#[async_trait]
impl Authenticator for ModularRealmAuthenticator {
    async fn authenticate(
        &self,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError> {
        if self.realms.is_empty() {
            return Err(AuthenticationError::Custom(
                "Configuration error: No realms have been configured!  One or more realms must be \
                 present to execute an authentication attempt."
                    .to_string(),
            ));
        }

        if self.realms.len() == 1 {
            self.do_single_realm_authentication(authentication_token)
                .await
        } else {
            self.do_multi_realm_authentication(authentication_token)
                .await
        }
    }
}

impl LogoutAware for ModularRealmAuthenticator {
    fn on_logout(&self, principals: &dyn PrincipalCollection) {
        // The realms of the port drop the authentication they cached through
        // their own `LogoutAware` implementation, which the security manager
        // calls for every realm it holds.
        if principals.is_empty() {
            warn!("Logging out a subject without a principal.");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ModularRealmAuthenticator;
    use crate::core::{
        authc::{
            authenticator::Authenticator,
            username_password_token::UsernamePasswordToken, AuthenticationError,
        },
        realm::{
            simple_account_realm::{SimpleAccount, SimpleAccountRealm},
            Realm,
        },
    };

    fn realm(name: &str, with_root: bool) -> SimpleAccountRealm {
        let mut realm = SimpleAccountRealm::with_name(name);
        if with_root {
            realm.add_account(SimpleAccount::new("root").with_password("secret"));
        }

        realm
    }

    fn token(username: &str, password: &str) -> UsernamePasswordToken {
        UsernamePasswordToken::new(username.to_string(), password.to_string(), false, None)
    }

    #[tokio::test]
    async fn a_single_realm_authenticates_the_token_it_supports() {
        let authenticator = ModularRealmAuthenticator::new(vec![
            Arc::new(realm("iniRealm", true)) as Arc<dyn Realm>
        ]);

        let info = authenticator
            .authenticate(&token("root", "secret"))
            .await
            .expect("the realm knows the account");

        assert_eq!(
            info.get_principals()
                .and_then(|principals| principals.get_primary_principal())
                .map(ToString::to_string),
            Some("root".to_string())
        );
    }

    #[tokio::test]
    async fn an_unknown_account_is_refused_by_the_realm_that_supports_it() {
        let authenticator = ModularRealmAuthenticator::new(vec![
            Arc::new(realm("iniRealm", false)) as Arc<dyn Realm>
        ]);

        let error = authenticator
            .authenticate(&token("root", "secret"))
            .await
            .err()
            .expect("the realm knows no account");

        assert!(
            matches!(error, AuthenticationError::Custom(message) if message.contains("iniRealm"))
        );
    }

    #[tokio::test]
    async fn a_configuration_without_a_realm_is_refused() {
        let authenticator = ModularRealmAuthenticator::default();

        let error = authenticator
            .authenticate(&token("root", "secret"))
            .await
            .err()
            .expect("there is no realm to ask");

        assert!(
            matches!(error, AuthenticationError::Custom(message) if message.contains("No realms"))
        );
        assert!(authenticator.is_empty());
    }

    #[tokio::test]
    async fn a_realm_that_does_not_support_the_token_leaves_it_unauthenticated() {
        let authenticator = ModularRealmAuthenticator::new(vec![
            Arc::new(realm("iniRealm", true)) as Arc<dyn Realm>
        ]);
        let bearer = crate::core::authc::bearer_token::BearerToken::new("a-bearer-token".to_string(), None);

        let error = authenticator
            .authenticate(&bearer)
            .await
            .err()
            .expect("the account realm does not handle a bearer token");

        assert!(matches!(error, AuthenticationError::Unknown));
    }

    #[tokio::test]
    async fn several_realms_merge_the_principals_they_authenticate() {
        let authenticator = ModularRealmAuthenticator::new(vec![
            Arc::new(realm("firstRealm", true)) as Arc<dyn Realm>,
            Arc::new(realm("secondRealm", true)) as Arc<dyn Realm>,
        ]);

        let info = authenticator
            .authenticate(&token("root", "secret"))
            .await
            .expect("both realms know the account");

        let principals = info.get_principals().expect("the info names its principals");
        let realm_names = principals.get_realm_names().expect("the realms are named");
        assert_eq!(realm_names.len(), 2);
        assert!(realm_names.contains("firstRealm"));
        assert!(realm_names.contains("secondRealm"));
    }

    #[tokio::test]
    async fn a_strategy_that_requires_every_realm_refuses_a_partial_login() {
        use crate::core::authc::pam::authentication_strategy::AllSuccessfulStrategy;

        let authenticator = ModularRealmAuthenticator::new(vec![
            Arc::new(realm("firstRealm", true)) as Arc<dyn Realm>,
            Arc::new(realm("secondRealm", false)) as Arc<dyn Realm>,
        ])
        .with_authentication_strategy(Arc::new(AllSuccessfulStrategy));

        let error = authenticator
            .authenticate(&token("root", "secret"))
            .await
            .err()
            .expect("one realm did not authenticate the account");

        assert!(matches!(error, AuthenticationError::Custom(message) if message.contains("secondRealm")));
    }

    #[tokio::test]
    async fn a_wrong_password_is_refused_by_the_account_realm() {
        let authenticator =
            ModularRealmAuthenticator::new(vec![Arc::new(realm("iniRealm", true)) as Arc<dyn Realm>]);

        let error = authenticator
            .authenticate(&token("root", "wrong"))
            .await
            .err()
            .expect("the password is not the stored one");

        assert!(matches!(error, AuthenticationError::Custom(message) if message.contains("iniRealm")));
    }

    #[tokio::test]
    async fn a_token_no_realm_authenticates_is_refused() {
        let authenticator = ModularRealmAuthenticator::new(vec![
            Arc::new(realm("firstRealm", false)) as Arc<dyn Realm>,
            Arc::new(realm("secondRealm", false)) as Arc<dyn Realm>,
        ]);

        let error = authenticator
            .authenticate(&token("nobody", "secret"))
            .await
            .err()
            .expect("no realm knows the account");

        assert!(
            matches!(error, AuthenticationError::Custom(message) if message.to_lowercase().contains("no realm"))
        );
    }
}
