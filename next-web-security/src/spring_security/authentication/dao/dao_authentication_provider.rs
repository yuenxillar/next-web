use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{async_trait, ArcSwap};
use tracing::debug;

use crate::{
    authentication::{
        dao::base_user_details_authentication_provider::BaseUserDetailsAuthenticationProviderExt,
        dao::BaseUserDetailsAuthenticationProvider, password::CompromisedPasswordChecker,
        UsernamePasswordAuthenticationToken,
    },
    core::{
        userdetails::{UserDetails, UserDetailsPasswordService, UserDetailsService},
        Authentication, AuthenticationError, AuthenticationErrorKind,
    },
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
    web::authentication::AuthPrincipal,
};

/// An AuthenticationProvider implementation that retrieves user details from a UserDetailsService.
pub struct DaoAuthenticationProvider {
    password_encoder: Arc<dyn PasswordEncoder>,
    user_not_found_encoded_password: ArcSwap<Option<String>>,
    user_details_service: Arc<dyn UserDetailsService>,
    user_details_password_service: Option<Arc<dyn UserDetailsPasswordService>>,
    compromised_password_checker: Option<Arc<dyn CompromisedPasswordChecker>>,

    base: BaseUserDetailsAuthenticationProvider,
}

impl DaoAuthenticationProvider {
    /// The plaintext password used to perform PasswordEncoder.matches(CharSequence, String) on when the user is not found to avoid SEC-2056.
    const USER_NOT_FOUND_PASSWORD: &str = "userNotFoundPassword";

    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_details_service,
            password_encoder: Arc::new(BCryptPasswordEncoder::default()),
            user_not_found_encoded_password: ArcSwap::from_pointee(None),
            user_details_password_service: None,
            compromised_password_checker: None,

            base: Default::default(),
        }
    }

    /// Sets the PasswordEncoder instance to be used to encode and validate passwords.
    ///  If not set, the password will be compared using PasswordEncoderFactories.create_delegating_password_encoder()
    pub fn set_password_encoder(&mut self, password_encoder: Arc<dyn PasswordEncoder>) {
        self.password_encoder = password_encoder;
        self.user_not_found_encoded_password.store(Arc::new(None));
    }

    pub fn password_encoder(&self) -> &dyn PasswordEncoder {
        self.password_encoder.as_ref()
    }

    pub fn user_details_service(&self) -> &dyn UserDetailsService {
        self.user_details_service.as_ref()
    }

    pub fn set_user_details_password_service(
        &mut self,
        user_details_password_service: Arc<dyn UserDetailsPasswordService>,
    ) {
        self.user_details_password_service = Some(user_details_password_service);
    }

    /// Sets the CompromisedPasswordChecker to be used before creating a successful authentication. Defaults to none.
    pub fn set_compromised_password_checker(
        &mut self,
        compromised_password_checker: Arc<dyn CompromisedPasswordChecker>,
    ) {
        self.compromised_password_checker = Some(compromised_password_checker);
    }

    async fn additional_authentication_checks(
        &self,
        user_details: Arc<dyn UserDetails>,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<(), AuthenticationError> {
        let presented_password = match authentication.credentials() {
            Some(credentials) => credentials.to_string(),
            None => {
                debug!("Failed to authenticate since no credentials provided");
                return Err(AuthenticationError::with_kind(
                    self.messages.message_or_default(
                        "BaseUserDetailsAuthenticationProvider.badCredentials",
                        None,
                        "Bad credentials",
                    ),
                    AuthenticationErrorKind::BadCredentials,
                ));
            }
        };

        let matches = user_details
            .password()
            .map(|password| self.password_encoder.matches(&presented_password, password))
            .unwrap_or(false);
        if !matches {
            debug!("Failed to authenticate since password does not match stored value");
            return Err(AuthenticationError::with_kind(
                self.messages.message_or_default(
                    "BaseUserDetailsAuthenticationProvider.badCredentials",
                    None,
                    "Bad credentials",
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        Ok(())
    }

    async fn retrieve_user(
        &self,
        username: &str,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        self.prepare_timing_attack_protection()?;

        match self
            .user_details_service
            .load_user_by_username(username)
            .await
        {
            Ok(user) => Ok(user),
            Err(err) => {
                if err.kind() == AuthenticationErrorKind::UsernameNotFound {
                    self.mitigate_against_timing_attack(authentication);
                }

                Err(err)
            }
        }
    }

    async fn create_success_authentication(
        &self,
        principal: AuthPrincipal,
        authentication: &dyn Authentication,
        user: Arc<dyn UserDetails>,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let presented_password = match authentication.credentials() {
            Some(val) => val.to_string(),
            None => {
                return Err(AuthenticationError::new(
                    "Authentication.credentials() cannot be none",
                ))
            }
        };
        let is_password_compromised = self
            .compromised_password_checker
            .as_ref()
            .map(|checker| checker.check(Some(&presented_password)).is_compromised())
            .unwrap_or_default();

        if is_password_compromised {
            return Err(AuthenticationError::with_kind(
                "The provided password is compromised, please change your password",
                AuthenticationErrorKind::CompromisedPassword,
            ));
        }

        let existing_encoded_password = user.password();
        let upgrade_encoding = if self.user_details_password_service.is_some() {
            match existing_encoded_password {
                Some(password) => self
                    .password_encoder
                    .upgrade_encoding(password)
                    .map_err(|err| AuthenticationError::new(err.to_string()))?,
                None => false,
            }
        } else {
            false
        };

        let mut _user = None;
        if upgrade_encoding {
            let new_password = self
                .password_encoder
                .as_ref()
                .encode(Some(&presented_password))
                .map_err(|err| AuthenticationError::new(err.to_string()))?;
            if let Some(password_service) = &self.user_details_password_service {
                _user = Some(
                    password_service
                        .update_password(user.clone(), new_password)
                        .await,
                );
            }
        }

        Ok(self.base.create_success_authentication(
            principal,
            authentication,
            _user.as_deref().unwrap_or(user.as_ref()),
        ))
    }

    fn prepare_timing_attack_protection(&self) -> Result<(), AuthenticationError> {
        if self.user_not_found_encoded_password.load().is_none() {
            let password = self
                .password_encoder
                .encode(Some(Self::USER_NOT_FOUND_PASSWORD))
                .map_err(|error| {
                    AuthenticationError::with_kind(
                        error.to_string(),
                        AuthenticationErrorKind::InternalAuthentication,
                    )
                })?;
            self.user_not_found_encoded_password
                .store(Arc::new(password));
        }

        Ok(())
    }

    fn mitigate_against_timing_attack(&self, authentication: &UsernamePasswordAuthenticationToken) {
        let Some(credentials) = authentication.credentials() else {
            return;
        };

        match self.user_not_found_encoded_password.load().as_deref() {
            Some(password) => {
                let presented_password = credentials.to_string();
                self.password_encoder.matches(&presented_password, password);
            }
            None => return,
        }
    }
}

#[async_trait]
impl BaseUserDetailsAuthenticationProviderExt for DaoAuthenticationProvider {
    async fn additional_authentication_checks(
        &self,
        user_details: Arc<dyn UserDetails>,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<(), AuthenticationError> {
        DaoAuthenticationProvider::additional_authentication_checks(
            self,
            user_details,
            authentication,
        )
        .await
    }

    async fn retrieve_user(
        &self,
        username: &str,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        DaoAuthenticationProvider::retrieve_user(self, username, authentication).await
    }

    async fn create_success_authentication(
        &self,
        principal: AuthPrincipal,
        authentication: &dyn Authentication,
        user: Arc<dyn UserDetails>,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        DaoAuthenticationProvider::create_success_authentication(
            self,
            principal,
            authentication,
            user,
        )
        .await
    }
}

impl Deref for DaoAuthenticationProvider {
    type Target = BaseUserDetailsAuthenticationProvider;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DaoAuthenticationProvider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        authentication::{dao::UserDetailsPrincipal, AuthenticationProvider},
        core::{
            authority::{FactorGrantedAuthority, SimpleGrantedAuthority},
            userdetails::{MapUserDetailsService, User},
        },
        web::authentication::AuthPrincipal,
    };
    use next_web_core::error::BoxError;

    #[derive(Clone)]
    struct PlainTextPasswordEncoder;

    impl PasswordEncoder for PlainTextPasswordEncoder {
        fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError> {
            Ok(raw_password.map(str::to_owned))
        }

        fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
            raw_password == encoded_password
        }
    }

    fn authentication(username: &str, password: &str) -> Arc<dyn Authentication> {
        let principal: AuthPrincipal = Arc::new(username.to_owned());
        let credentials: AuthPrincipal = Arc::new(password.to_owned());
        Arc::new(UsernamePasswordAuthenticationToken::unauthenticated(
            Some(principal),
            Some(credentials),
        ))
    }

    fn provider() -> DaoAuthenticationProvider {
        let user: Arc<dyn UserDetails> = Arc::new(User::new(
            "user",
            Some("password".to_owned()),
            vec![Arc::new(SimpleGrantedAuthority::new("ROLE_USER"))],
        ));
        let service = Arc::new(MapUserDetailsService::with_users(vec![user]));
        let mut provider = DaoAuthenticationProvider::new(service);
        provider.set_password_encoder(Arc::new(PlainTextPasswordEncoder));
        provider
    }

    #[tokio::test]
    async fn authenticates_valid_credentials_and_adds_password_factor() {
        let result = provider()
            .authenticate(&authentication("user", "password"))
            .await
            .expect("authentication should not fail")
            .expect("provider should support the token");

        assert!(result.is_authenticated());
        assert_eq!(result.name(), "user");
        assert!(result
            .principal()
            .and_then(|principal| principal.as_any().downcast_ref::<UserDetailsPrincipal>())
            .is_some());
        assert!(result.authorities().iter().any(|authority| {
            authority.authority() == Some(FactorGrantedAuthority::PASSWORD_AUTHORITY)
        }));
    }

    #[tokio::test]
    async fn rejects_invalid_credentials() {
        let error = provider()
            .authenticate(&authentication("user", "wrong"))
            .await
            .expect_err("invalid password should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }

    #[tokio::test]
    async fn hides_username_not_found_by_default() {
        let service = Arc::new(MapUserDetailsService::default());
        let mut provider = DaoAuthenticationProvider::new(service);
        provider.set_password_encoder(Arc::new(PlainTextPasswordEncoder));

        let error = provider
            .authenticate(&authentication("missing", "password"))
            .await
            .expect_err("missing user should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }
}
