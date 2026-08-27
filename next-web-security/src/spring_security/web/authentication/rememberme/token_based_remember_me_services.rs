use std::{
    error::Error,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::{time::current_time_millis, StringUtils},
};
use tracing::debug;

use crate::{
    core::{
        userdetails::{UserDetails, UserDetailsService},
        Authentication,
    },
    web::authentication::{
        logout::LogoutHandler,
        rememberme::base_remember_me_services::{
            BaseRememberMeServices, BaseRememberMeServicesExt,
        },
    },
};

const DEFAULT_MATCHING_ALGORITHM: RememberMeTokenAlgorithm = RememberMeTokenAlgorithm::SHA256;
const DEFAULT_ENCODING_ALGORITHM: RememberMeTokenAlgorithm = RememberMeTokenAlgorithm::SHA256;

/// Identifies previously remembered users by a Base-64 encoded cookie.
///
/// This implementation does not rely on an external database, so is attractive for simple
/// applications. The cookie will be valid for a specific period from the date of the last
/// `login_success` call. As per the interface contract, this method will only be called
/// when the principal completes a successful interactive authentication. As such the time
/// period commences from the last authentication attempt where they furnished credentials -
/// not the time period they last logged in via remember-me. The implementation will only
/// send a remember-me token if the parameter defined by `set_parameter` is present.
///
/// A `UserDetailsService` is required by this implementation, so that it can construct a
/// valid `Authentication` from the returned `UserDetails`. This is also necessary so that
/// the user's password is available and can be checked as part of the encoded cookie.
///
/// The cookie encoded by this implementation adopts the following form:
///
/// ```text
/// username + ":" + expiryTime + ":" + algorithmName + ":"
///     + algorithmHex(username + ":" + expiryTime + ":" + password + ":" + key)
/// ```
///
/// This implementation uses the algorithm configured in `encoding_algorithm` to
/// encode the signature. It will try to use the algorithm retrieved from the
/// `algorithm_name` to validate the signature. However, if the `algorithm_name`
/// is not present in the cookie value, the algorithm configured in
/// `matching_algorithm` will be used to validate the signature. This allows users to
/// safely upgrade to a different encoding algorithm while still able to verify old ones if
/// there is no `algorithm_name` present.
///
/// As such, if the user changes their password, any remember-me token will be invalidated.
/// Equally, the system administrator may invalidate every remember-me token on issue by
/// changing the key. This provides some reasonable approaches to recovering from a
/// remember-me token being left on a public machine (e.g. kiosk system, Internet cafe
/// etc). Most importantly, at no time is the user's password ever sent to the user agent,
/// providing an important security safeguard. Unfortunately the username is necessary in
/// this implementation (as we do not want to rely on a database for remember-me services).
/// High security applications should be aware of this occasionally undesired disclosure of
/// a valid username.
///
/// This is a basic remember-me implementation which is suitable for many applications.
/// However, we recommend a database-based implementation if you require a more secure
/// remember-me approach.
///
/// By default the tokens will be valid for 14 days from the last successful authentication
/// attempt. This can be changed using `set_token_validity_seconds`. If this value
/// is less than zero, the `expiry_time` will remain at 14 days, but the negative
/// value will be used for the `max_age` property of the cookie, meaning that it will
/// not be stored when the browser is closed.
#[derive(Clone)]
pub struct TokenBasedRememberMeServices {
    encoding_algorithm: RememberMeTokenAlgorithm,
    matching_algorithm: RememberMeTokenAlgorithm,

    base: BaseRememberMeServices,
}

impl TokenBasedRememberMeServices {
    pub fn new(key: impl Into<String>, user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self::with_algorithm(key, user_details_service, DEFAULT_ENCODING_ALGORITHM)
    }

    /// Construct the instance with the parameters provided
    /// * `key` - the signature key
    /// * `user_details_service` - the `UserDetailsService`
    /// * `encoding_algorithm` - the `RememberMeTokenAlgorithm` used to encode the signature
    pub fn with_algorithm(
        key: impl Into<String>,
        user_details_service: Arc<dyn UserDetailsService>,
        encoding_algorithm: RememberMeTokenAlgorithm,
    ) -> Self {
        Self {
            encoding_algorithm,
            matching_algorithm: DEFAULT_MATCHING_ALGORITHM,

            base: BaseRememberMeServices::new(key, user_details_service),
        }
    }

    fn is_valid_cookie_tokens_length(&self, cookie_tokens: &[String]) -> bool {
        cookie_tokens.len() == 3 || cookie_tokens.len() == 4
    }

    fn get_token_expiry_time(&self, cookie_tokens: &[String]) -> Result<i64, String> {
        cookie_tokens[1].parse::<i64>().map_err(|_| {
            format!(
                "Cookie token[1] did not contain a valid number (contained '{}')",
                cookie_tokens[1]
            )
        })
    }

    /// Calculates the digital signature to be put in the cookie. Default value is
    /// `encoding_algorithm` applied to ("username:tokenExpiryTime:password:key")
    fn make_token_signature(
        &self,
        token_expiry_time: i64,
        username: &str,
        password: &str,
    ) -> String {
        let data = format!(
            "{}:{}:{}:{}",
            username,
            token_expiry_time,
            password,
            self.get_key()
        );
        self.digest_data(&data, self.encoding_algorithm)
    }

    /// Calculates the digital signature to be put in the cookie.
    fn make_token_signature_with_algorithm(
        &self,
        token_expiry_time: i64,
        username: &str,
        password: Option<&str>,
        algorithm: RememberMeTokenAlgorithm,
    ) -> String {
        let password = password.unwrap_or("");
        let data = format!(
            "{}:{}:{}:{}",
            username,
            token_expiry_time,
            password,
            self.get_key()
        );
        self.digest_data(&data, algorithm)
    }

    fn digest_data(&self, data: &str, algorithm: RememberMeTokenAlgorithm) -> String {
        let digest = algorithm.digest(data.as_bytes());
        hex::encode(digest)
    }

    fn is_token_expired(&self, token_expiry_time: i64) -> bool {
        token_expiry_time < current_time_millis()
    }

    /// Sets the algorithm to be used to match the token signature
    pub fn set_matching_algorithm(&mut self, matching_algorithm: RememberMeTokenAlgorithm) {
        self.matching_algorithm = matching_algorithm;
    }

    // Calculates the validity period in seconds for a newly generated remember-me login.
    /// After this period (from the current time) the remember-me login will be considered
    /// expired. This method allows customization based on request parameters supplied with
    /// the login or information in the `Authentication` object. The default value
    /// is just the token validity period property, `token_validity_seconds`.
    ///
    /// The returned value will be used to work out the expiry time of the token and will
    /// also be used to set the `max_age` property of the cookie.
    ///
    /// See SEC-485.
    fn calculate_login_lifetime(&self, _authentication: &dyn Authentication) -> i32 {
        self.get_token_validity_seconds()
    }

    fn retrieve_user_name(&self, authentication: &dyn Authentication) -> String {
        if let Some(user_details) = authentication
            .principal()
            .and_then(|s| s.downcast_ref::<Arc<dyn UserDetails>>())
        {
            return user_details.username().to_string();
        }

        authentication
            .principal()
            .and_then(|s| s.downcast_ref::<String>().map(ToOwned::to_owned))
            .unwrap_or_default()
    }

    fn retrieve_password(&self, authentication: &dyn Authentication) -> Option<String> {
        if let Some(user_details) = authentication
            .principal()
            .and_then(|s| s.downcast_ref::<Arc<dyn UserDetails>>())
        {
            return user_details.password().map(ToString::to_string);
        }
        authentication
            .credentials()
            .and_then(|s| s.downcast_ref::<String>().map(ToOwned::to_owned))
    }

    /// Constant time comparison to prevent against timing attacks.
    fn constant_time_equals(expected: &str, actual: &str) -> bool {
        let expected_bytes = expected.as_bytes();
        let actual_bytes = actual.as_bytes();

        if expected_bytes.len() != actual_bytes.len() {
            return false;
        }

        let mut result: u8 = 0;
        for (a, b) in expected_bytes.iter().zip(actual_bytes.iter()) {
            result |= a ^ b;
        }
        result == 0
    }
}

#[async_trait]
impl LogoutHandler for TokenBasedRememberMeServices {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        self.base.logout(request, response, authentication).await
    }
}

#[async_trait]
impl BaseRememberMeServicesExt for TokenBasedRememberMeServices {
    async fn on_login_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        successful_authentication: &dyn Authentication,
    ) {
        let username = self.retrieve_user_name(successful_authentication);
        let mut password = self.retrieve_password(successful_authentication);

        // If unable to find a username and password, just abort as
        // TokenBasedRememberMeServices is unable to construct a valid token in this case.
        if !StringUtils::has_text(&username) {
            debug!("Unable to retrieve username");
            return;
        }

        if password.as_deref().unwrap_or("").is_empty() {
            if let Some(user) = self
                .get_user_details_service()
                .load_user_by_username(&username)
                .await
                .ok()
            {
                password = user.password().map(ToString::to_string);
                if password.as_deref().map(str::is_empty).unwrap_or(true) {
                    debug!("Unable to obtain password for user: {}", username);
                    return;
                }
            } else {
                debug!("Unable to obtain password for user: {}", username);
                return;
            }
        }

        let token_lifetime = self.calculate_login_lifetime(successful_authentication);
        let mut expiry_time = current_time_millis();
        // SEC-949
        expiry_time += 1000
            * (if token_lifetime < 0 {
                BaseRememberMeServices::TWO_WEEKS_S
            } else {
                token_lifetime
            }) as i64;

        let signature_value = self.make_token_signature_with_algorithm(
            expiry_time,
            &username,
            password.as_deref(),
            self.encoding_algorithm,
        );

        let cookie_tokens = vec![
            username.to_string(),
            expiry_time.to_string(),
            self.encoding_algorithm.name().to_string(),
            signature_value,
        ];

        self.set_cookie(&cookie_tokens, token_lifetime, request, response);

        debug!(
            "Added remember-me cookie for user '{}', expiry: '{}'",
            username,
            format_timestamp(expiry_time)
        );
    }

    async fn process_auto_login_cookie(
        &self,
        cookie_tokens: &[String],
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<Arc<dyn UserDetails>, Box<dyn Error>> {
        if !self.is_valid_cookie_tokens_length(cookie_tokens) {
            return Err(format!(
                "Cookie token did not contain 3 or 4 tokens, but contained '{:?}'",
                cookie_tokens
            )
            .into());
        }

        let token_expiry_time = self.get_token_expiry_time(cookie_tokens)?;

        if self.is_token_expired(token_expiry_time) {
            return Err(format!(
                "Cookie token[1] has expired (expired on '{}'; current time is '{}')",
                format_timestamp(token_expiry_time),
                format_timestamp(current_time_millis())
            )
            .into());
        }

        // Check the user exists. Defer lookup until after expiry time checked, to
        // possibly avoid expensive database call.
        let user_details = self
            .get_user_details_service()
            .load_user_by_username(&cookie_tokens[0])
            .await
            .map_err(|_| {
                format!(
                    "UserDetailsService returned null for username {}. This is an interface contract violation",
                    cookie_tokens[0]
                )
            })?;

        // Check signature of token matches remaining details. Must do this after user
        // lookup, as we need the DAO-derived password. If efficiency was a major issue,
        // just add in a UserCache implementation, but recall that this method is usually
        // only called once per HttpSession - if the token is valid, it will cause
        // SecurityContextHolder population, whilst if invalid, will cause the cookie to
        // be cancelled.
        let mut actual_token_signature = cookie_tokens[2].clone();
        let mut actual_algorithm = self.matching_algorithm;

        // If the cookie value contains the algorithm, we use that algorithm to check the signature
        if cookie_tokens.len() == 4 {
            actual_token_signature = cookie_tokens[3].clone();
            actual_algorithm = RememberMeTokenAlgorithm::from_str(&cookie_tokens[2])
                .map_err(|_| String::from("Invalid algorithm in cookie"))?;
        }

        let expected_token_signature = self.make_token_signature_with_algorithm(
            token_expiry_time,
            user_details.username(),
            user_details.password(),
            actual_algorithm,
        );

        if !Self::constant_time_equals(&expected_token_signature, &actual_token_signature) {
            return Err(format!(
                "Cookie contained signature '{}' but expected '{}'",
                actual_token_signature, expected_token_signature
            )
            .into());
        }

        Ok(user_details)
    }
}

impl Deref for TokenBasedRememberMeServices {
    type Target = BaseRememberMeServices;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for TokenBasedRememberMeServices {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Represents the algorithm used for remember-me token signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RememberMeTokenAlgorithm {
    MD5,
    SHA256,
}

impl RememberMeTokenAlgorithm {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "MD5" => Ok(Self::MD5),
            "SHA-256" => Ok(Self::SHA256),
            _ => Err(format!("Unknown algorithm: {}", s)),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::MD5 => "MD5",
            Self::SHA256 => "SHA-256",
        }
    }

    fn digest(&self, data: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        match self {
            Self::MD5 => {
                // Note: MD5 is provided here for backward compatibility,
                // but SHA-256 should be used for new tokens
                md5::compute(data).to_vec()
            }
            Self::SHA256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
        }
    }
}

fn format_timestamp(millis: i64) -> String {
    // This would format the timestamp as a human-readable date string
    // Implementation depends on your date/time library
    format!("{}ms since epoch", millis)
}
