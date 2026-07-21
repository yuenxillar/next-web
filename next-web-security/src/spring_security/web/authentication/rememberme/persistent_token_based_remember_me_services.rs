use std::{
    error::Error,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::time::current_time_millis,
};
use rand::{rngs::OsRng, RngCore};
use tracing::{debug, error};

use crate::{
    core::{
        userdetails::{UserDetails, UserDetailsService},
        Authentication,
    },
    web::authentication::{
        logout::LogoutHandler,
        rememberme::{
            base_remember_me_services::BaseRememberMeServices, BaseRememberMeServicesExt,
            PersistentRememberMeToken, PersistentTokenRepository,
        },
    },
};

/// `RememberMeServices` implementation based on Barry Jaspan's
/// [Improved Persistent Login Cookie Best Practice](https://web.archive.org/web/20180819014446/http://jaspan.com/improved_persistent_login_cookie_best_practice).
///
/// There is a slight modification to the described approach, in that the username is not
/// stored as part of the cookie but obtained from the persistent store via an
/// implementation of `PersistentTokenRepository`. The latter should place a unique
/// constraint on the series identifier, so that it is impossible for the same identifier
/// to be allocated to two different users.
///
/// User management such as changing passwords, removing users and setting user status
/// should be combined with maintenance of the user's persistent tokens.
///
/// Note that while this class will use the date a token was created to check whether a
/// presented cookie is older than the configured `token_validity_seconds` property
/// and deny authentication in this case, it will not delete these tokens from storage. A
/// suitable batch process should be run periodically to remove expired tokens from the
/// database.
#[derive(Clone)]
pub struct PersistentTokenBasedRememberMeServices {
    token_repository: Arc<dyn PersistentTokenRepository>,
    series_length: usize,
    token_length: usize,

    inner: BaseRememberMeServices,
}

impl PersistentTokenBasedRememberMeServices {
    pub const DEFAULT_SERIES_LENGTH: usize = 16;
    pub const DEFAULT_TOKEN_LENGTH: usize = 16;

    pub fn new(
        key: impl Into<String>,
        user_details_service: Arc<dyn UserDetailsService>,
        token_repository: Arc<dyn PersistentTokenRepository>,
    ) -> Self {
        Self {
            token_repository,
            series_length: Self::DEFAULT_SERIES_LENGTH,
            token_length: Self::DEFAULT_TOKEN_LENGTH,

            inner: BaseRememberMeServices::new(key, user_details_service),
        }
    }

    fn generate_series_data(&self) -> String {
        let mut new_series = vec![0u8; self.series_length];
        OsRng.fill_bytes(&mut new_series);
        STANDARD.encode(&new_series)
    }

    fn generate_token_data(&self) -> String {
        let mut new_token = vec![0u8; self.token_length];
        OsRng.fill_bytes(&mut new_token);
        STANDARD.encode(&new_token)
    }

    fn add_cookie(
        &self,
        token: &PersistentRememberMeToken,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        let cookie_values = vec![
            token.get_series().to_string(),
            token.get_token_value().to_string(),
        ];
        self.set_cookie(
            &cookie_values,
            self.get_token_validity_seconds(),
            request,
            response,
        );
    }

    pub fn set_series_length(&mut self, series_length: usize) {
        self.series_length = series_length;
    }

    pub fn set_token_length(&mut self, token_length: usize) {
        self.token_length = token_length;
    }

    fn set_token_validity_seconds(&mut self, token_validity_seconds: i32) {
        assert!(
            token_validity_seconds > 0,
            "tokenValiditySeconds must be positive for this implementation"
        );
        self.inner
            .set_token_validity_seconds(token_validity_seconds);
    }
}

#[async_trait]
impl BaseRememberMeServicesExt for PersistentTokenBasedRememberMeServices {
    /// Creates a new persistent login token with a new series number,
    /// stores the data in the persistent token repository and adds the corresponding cookie to the response.
    async fn on_login_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        successful_authentication: &dyn Authentication,
    ) {
        let username = successful_authentication.name();
        debug!("Creating new persistent login for user {}", username);

        let persistent_token = PersistentRememberMeToken::new(
            username,
            self.generate_series_data(),
            self.generate_token_data(),
            current_time_millis(),
        );

        self.token_repository
            .create_new_token(&persistent_token)
            .inspect_err(|err| error!("Failed to save persistent token: {}", err))
            .ok();
        self.add_cookie(&persistent_token, request, response);
    }

    async fn process_auto_login_cookie(
        &self,
        cookie_tokens: &[String],
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<Arc<dyn UserDetails>, Box<dyn Error>> {
        if cookie_tokens.len() != 2 {
            return Err(format!(
                "Cookie token did not contain 2 tokens, but contained '{:?}'",
                cookie_tokens
            )
            .into());
        }

        let presented_series = &cookie_tokens[0];
        let presented_token = &cookie_tokens[1];

        let token = self
            .token_repository
            .get_token_for_series(presented_series)
            .ok_or_else(|| {
                format!(
                    "No persistent token found for series id: {}",
                    presented_series
                )
            })?;

        // We have a match for this user/series combination
        if presented_token != token.get_token_value() {
            // Token doesn't match series value. Delete all logins for this user and throw
            // an exception to warn them.
            self.token_repository
                .remove_user_tokens(token.get_username());

            return Err(
                "Invalid remember-me token (Series/token) mismatch. Implies previous cookie theft attack."
                    .into(),
            );
        }

        if token.get_date() + (self.get_token_validity_seconds() as i64 * 1000)
            < current_time_millis()
        {
            return Err("Remember-me login has expired".into());
        }

        // Token also matches, so login is valid. Update the token value, keeping the
        // *same* series number.
        debug!(
            "Refreshing persistent login token for user '{}', series '{}'",
            token.get_username(),
            token.get_series()
        );

        let new_token = PersistentRememberMeToken::new(
            token.get_username(),
            token.get_series(),
            self.generate_token_data(),
            current_time_millis(),
        );

        self.token_repository
            .update_token(
                new_token.get_series(),
                new_token.get_token_value(),
                new_token.get_date(),
            )
            .map_err(|e| {
                error!("Failed to update token: {}", e);
                "Autologin failed due to data access problem"
            })?;

        self.add_cookie(&new_token, request, response);

        self.get_user_details_service()
            .load_user_by_username(token.get_username())
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl LogoutHandler for PersistentTokenBasedRememberMeServices {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        self.inner.logout(request, response, authentication).await;

        if let Some(auth) = authentication {
            self.token_repository.remove_user_tokens(auth.name())
        }
    }
}

impl Deref for PersistentTokenBasedRememberMeServices {
    type Target = BaseRememberMeServices;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PersistentTokenBasedRememberMeServices {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
