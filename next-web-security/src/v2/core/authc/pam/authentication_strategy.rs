//! Shiro's authentication strategies: what a multi-realm configuration requires
//! of the realms it asks.

use crate::core::authc::{authentication_info::AuthenticationInfo, AuthenticationError};

/// What a configuration of several realms requires of them.
///
/// The strategy is asked before the realms are consulted, after each of them
/// answered, and once they all did, so it can refuse a login that only one realm
/// authenticated, or one that any realm refused.
pub trait AuthenticationStrategy: Send + Sync {
    /// Returns the name of the strategy.
    fn name(&self) -> &'static str;

    /// Called before the first realm is asked.
    ///
    /// # Arguments
    ///
    /// * `realm_names` - The realms that support the token, in the order they are
    ///   asked.
    fn before_all_attempts(&self, realm_names: &[&str]) {
        let _ = realm_names;
    }

    /// Returns whether the realm is asked.
    ///
    /// # Arguments
    ///
    /// * `realm_name` - The realm that is about to be asked.
    fn before_attempt(&self, realm_name: &str) -> bool {
        let _ = realm_name;

        true
    }

    /// Returns whether the realms that have not answered yet are asked at all.
    ///
    /// A strategy that needs one success stops the calls as soon as it has one,
    /// which is what Shiro's `FirstSuccessfulStrategy` means by its name: the
    /// realms after the first that authenticated are not consulted.
    ///
    /// # Arguments
    ///
    /// * `authenticated_realms` - The realms that authenticated the token so far.
    fn should_continue(&self, authenticated_realms: &[String]) -> bool {
        let _ = authenticated_realms;

        true
    }

    /// Called with the answer of a realm.
    ///
    /// # Arguments
    ///
    /// * `realm_name` - The realm that answered.
    /// * `info` - The authentication of the realm, when it authenticated the token.
    ///
    /// # Errors
    ///
    /// Returns the error that ends the login when the answer is not enough for
    /// this strategy.
    fn after_attempt(
        &self,
        realm_name: &str,
        info: Option<&dyn AuthenticationInfo>,
    ) -> Result<(), AuthenticationError>;

    /// Called once every realm answered.
    ///
    /// # Arguments
    ///
    /// * `authenticated_realms` - The realms that authenticated the token.
    ///
    /// # Errors
    ///
    /// Returns the error that ends the login when the answers are not enough for
    /// this strategy.
    fn after_all_attempts(
        &self,
        authenticated_realms: &[String],
    ) -> Result<(), AuthenticationError>;
}

/// A strategy that is satisfied when at least one realm authenticated the token.
///
/// It is the default of Shiro and of the port: a deployment that adds a realm for
/// a directory trusts the directory and its own accounts alike.
#[derive(Clone, Copy, Debug, Default)]
pub struct AtLeastOneSuccessfulStrategy;

impl AuthenticationStrategy for AtLeastOneSuccessfulStrategy {
    fn name(&self) -> &'static str {
        "AtLeastOneSuccessfulStrategy"
    }

    fn after_attempt(
        &self,
        _realm_name: &str,
        _info: Option<&dyn AuthenticationInfo>,
    ) -> Result<(), AuthenticationError> {
        Ok(())
    }

    fn after_all_attempts(
        &self,
        authenticated_realms: &[String],
    ) -> Result<(), AuthenticationError> {
        if authenticated_realms.is_empty() {
            return Err(AuthenticationError::Custom(
                "Authentication token of unknown type combined with multiple realms: no realm \
                 authenticated the token."
                    .to_string(),
            ));
        }

        Ok(())
    }
}

/// A strategy that requires every realm that supports the token to authenticate it.
///
/// It is the strategy of a deployment whose realms are all sources of the same
/// identity: a token that only one of them knows is a token that must not pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllSuccessfulStrategy;

impl AuthenticationStrategy for AllSuccessfulStrategy {
    fn name(&self) -> &'static str {
        "AllSuccessfulStrategy"
    }

    fn after_attempt(
        &self,
        realm_name: &str,
        info: Option<&dyn AuthenticationInfo>,
    ) -> Result<(), AuthenticationError> {
        match info {
            Some(_) => Ok(()),
            None => Err(AuthenticationError::Custom(format!(
                "The realm [{realm_name}] did not authenticate the token, and the configuration \
                 requires every realm to authenticate it."
            ))),
        }
    }

    fn after_all_attempts(
        &self,
        authenticated_realms: &[String],
    ) -> Result<(), AuthenticationError> {
        if authenticated_realms.is_empty() {
            return Err(AuthenticationError::Custom(
                "No realm authenticated the token, and the configuration requires every realm to \
                 authenticate it."
                    .to_string(),
            ));
        }

        Ok(())
    }
}

/// A strategy that stops at the first realm that authenticates the token.
///
/// It is the strategy of a deployment whose realms are ordered by preference 鈥?a
/// directory first, the local accounts second 鈥?so that the login costs one lookup
/// and not one per realm.
#[derive(Clone, Copy, Debug, Default)]
pub struct FirstSuccessfulStrategy;

impl AuthenticationStrategy for FirstSuccessfulStrategy {
    fn name(&self) -> &'static str {
        "FirstSuccessfulStrategy"
    }

    fn should_continue(&self, authenticated_realms: &[String]) -> bool {
        authenticated_realms.is_empty()
    }

    fn after_attempt(
        &self,
        _realm_name: &str,
        _info: Option<&dyn AuthenticationInfo>,
    ) -> Result<(), AuthenticationError> {
        Ok(())
    }

    fn after_all_attempts(
        &self,
        authenticated_realms: &[String],
    ) -> Result<(), AuthenticationError> {
        if authenticated_realms.is_empty() {
            return Err(AuthenticationError::Custom(
                "No realm authenticated the token.".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{AllSuccessfulStrategy, AtLeastOneSuccessfulStrategy, AuthenticationStrategy};
    use crate::core::{
        authc::simple_authentication_info::SimpleAuthenticationInfo,
        subject::simple_principal_collection::SimplePrincipalCollection,
        util::object::Object,
    };

    fn info() -> SimpleAuthenticationInfo {
        SimpleAuthenticationInfo::new(
            Arc::new(SimplePrincipalCollection::new(
                [Object::Str("root".to_string())],
                "realm",
            )),
            None,
        )
    }

    #[test]
    fn the_first_success_is_enough_and_stops_the_calls() {
        use super::FirstSuccessfulStrategy;

        let strategy = FirstSuccessfulStrategy;
        let info = info();

        assert!(strategy.should_continue(&[]), "the first realm is always asked");
        assert!(strategy.after_attempt("first", Some(&info)).is_ok());
        assert!(
            !strategy.should_continue(&["first".to_string()]),
            "the realms after the first success are not asked"
        );
        assert!(strategy
            .after_all_attempts(&["first".to_string()])
            .is_ok());
        assert!(strategy.after_all_attempts(&[]).is_err());
        assert_eq!(strategy.name(), "FirstSuccessfulStrategy");
    }

    #[test]
    fn a_strategy_that_needs_every_realm_never_stops_the_calls() {
        let strategy = AllSuccessfulStrategy;

        assert!(strategy.should_continue(&[]));
        assert!(strategy.should_continue(&["first".to_string()]));
        assert!(AtLeastOneSuccessfulStrategy.should_continue(&["first".to_string()]));
    }

    #[test]
    fn at_least_one_success_is_satisfied_by_one_realm() {
        let strategy = AtLeastOneSuccessfulStrategy;
        let info = info();

        assert!(strategy.after_attempt("first", Some(&info)).is_ok());
        assert!(strategy.after_attempt("second", None).is_ok());
        assert!(strategy
            .after_all_attempts(&["first".to_string()])
            .is_ok());
        assert!(strategy.after_all_attempts(&[]).is_err());
        assert_eq!(strategy.name(), "AtLeastOneSuccessfulStrategy");
    }

    #[test]
    fn all_success_requires_every_attempt_to_succeed() {
        let strategy = AllSuccessfulStrategy;
        let info = info();

        assert!(strategy.after_attempt("first", Some(&info)).is_ok());
        let error = strategy
            .after_attempt("second", None)
            .expect_err("the strategy refuses a realm that did not authenticate");
        assert!(error.to_string().contains("second"));
        assert!(strategy
            .after_all_attempts(&["first".to_string(), "second".to_string()])
            .is_ok());
        assert!(strategy.after_all_attempts(&[]).is_err());
        assert_eq!(strategy.name(), "AllSuccessfulStrategy");
    }
}
