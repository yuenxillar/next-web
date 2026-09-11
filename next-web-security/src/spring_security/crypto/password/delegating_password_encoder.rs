use std::{collections::HashMap, sync::Arc};

use next_web_core::error::BoxError;
use tracing::error;

use crate::crypto::password::PasswordEncoder;

/// Default id prefix that denotes the start of the id in an encoded password.
const DEFAULT_ID_PREFIX: &str = "{";

/// Default id suffix that denotes the end of the id in an encoded password.
const DEFAULT_ID_SUFFIX: &str = "}";

/// Message used when an id has no mapped [`PasswordEncoder`].
const NO_PASSWORD_ENCODER_MAPPED: &str = "There is no password encoder mapped for the id '%s'. \
Check your configuration to ensure it matches one of the registered encoders.";

/// Message used when a password has no encoding prefix and no default encoder is set.
const NO_PASSWORD_ENCODER_PREFIX: &str = "Given that there is no default password encoder configured, \
each password must have a password encoding prefix. Please either prefix this password with '{noop}' \
or set a default password encoder in `DelegatingPasswordEncoder`.";

/// Message used when a password has a malformed encoding prefix.
const MALFORMED_PASSWORD_ENCODER_PREFIX: &str = "The name of the password encoder is improperly \
formatted or incomplete. The format should be '%sENCODER%spassword'.";

/// A password encoder that delegates to another [`PasswordEncoder`] based upon a prefixed
/// identifier.
///
/// # Password Storage Format
///
/// The general format for a password is:
///
/// ```text
/// {id}encodedPassword
/// ```
///
/// Such that "id" is an identifier used to look up which [`PasswordEncoder`] should be
/// used and "encodedPassword" is the original encoded password for the selected
/// [`PasswordEncoder`]. The "id" must be at the beginning of the password, start with the
/// id prefix and end with the id suffix. Both the id prefix and id suffix can be
/// customized via [`Self::with_id_prefix_and_suffix`]. If the "id" cannot be found, the
/// "id" is treated as `None`.
///
/// For example, the following might be a list of passwords encoded using different ids.
/// All of the original passwords are "password".
///
/// ```text
/// {bcrypt}$2a$10$dXJ3SW6G7P50lGmMkkmwe.20cQQubK3.HZWzG3YB1tlRy.fqvM/BG
/// {noop}password
/// {pbkdf2}5d923b44a6d129f3ddf3e3c8d29412723dcbde72445e8ef6bf3b508fbf17fa4ed4d6b99ca763d8dc
/// {scrypt}$e0801$8bWJaSu2IKSn9Z9kM+TPXfOc/9bdYSrN1oD9qfVThWEwdRTnO7re7Ei+fUZRJ68k9lTyuTeUp4of4g24hHnazw==$OAOec05+bXxvuu/1qZ6NUR+xQYvYv7BeL1QxwRpY5Pc=
/// {sha256}97cde38028ad898ebc02e690819fa220e88c62e0699403e94fff291cfffaf8410849f27605abcbc0
/// ```
///
/// # Password Encoding
///
/// The `id_for_encode` passed into the constructor determines which [`PasswordEncoder`]
/// is used for encoding passwords. The result is prefixed with the id, for example
/// `{bcrypt}...`.
///
/// # Password Matching
///
/// Matching is done based upon the "id" and the mapping of the "id" to the
/// [`PasswordEncoder`] provided in the constructor.
///
/// By default, matching a password whose "id" is not mapped (including a missing id) will
/// log an error and return `false`. This behaviour can be customized using
/// [`Self::set_default_password_encoder_for_matches`].
#[derive(Clone)]
pub struct DelegatingPasswordEncoder {
    id_prefix: String,
    id_suffix: String,
    id_for_encode: String,
    password_encoder_for_encode: Arc<dyn PasswordEncoder>,
    id_to_password_encoder: HashMap<String, Arc<dyn PasswordEncoder>>,
    default_password_encoder_for_matches: Option<Arc<dyn PasswordEncoder>>,
}

impl DelegatingPasswordEncoder {
    /// Creates a new instance using the default id prefix `{` and id suffix `}`.
    ///
    /// # Arguments
    ///
    /// * `id_for_encode` - the id used to look up which [`PasswordEncoder`] should be used
    ///   for [`PasswordEncoder::encode`]
    /// * `id_to_password_encoder` - a map of id to [`PasswordEncoder`] used to determine
    ///   which [`PasswordEncoder`] should be used for [`PasswordEncoder::matches`]
    pub fn new(
        id_for_encode: impl Into<String>,
        id_to_password_encoder: HashMap<String, Arc<dyn PasswordEncoder>>,
    ) -> Self {
        Self::with_id_prefix_and_suffix(
            id_for_encode,
            id_to_password_encoder,
            DEFAULT_ID_PREFIX,
            DEFAULT_ID_SUFFIX,
        )
    }

    /// Creates a new instance with custom id prefix and suffix.
    ///
    /// # Arguments
    ///
    /// * `id_for_encode` - the id used to look up which [`PasswordEncoder`] should be used
    ///   for [`PasswordEncoder::encode`]
    /// * `id_to_password_encoder` - a map of id to [`PasswordEncoder`] used to determine
    ///   which [`PasswordEncoder`] should be used for [`PasswordEncoder::matches`]
    /// * `id_prefix` - the prefix that denotes the start of the id in the encoded results
    /// * `id_suffix` - the suffix that denotes the end of an id in the encoded results
    ///
    /// # Panics
    ///
    /// Panics if `id_for_encode` is empty, if `id_suffix` is empty, if `id_prefix`
    /// contains `id_suffix`, if `id_for_encode` is not present in `id_to_password_encoder`,
    /// or if any id contains the id prefix or id suffix. These checks are intended to run
    /// during application configuration.
    pub fn with_id_prefix_and_suffix(
        id_for_encode: impl Into<String>,
        id_to_password_encoder: HashMap<String, Arc<dyn PasswordEncoder>>,
        id_prefix: impl Into<String>,
        id_suffix: impl Into<String>,
    ) -> Self {
        let id_for_encode = id_for_encode.into();
        let id_prefix = id_prefix.into();
        let id_suffix = id_suffix.into();

        assert!(!id_for_encode.is_empty(), "idForEncode cannot be empty");
        assert!(!id_suffix.is_empty(), "suffix cannot be empty");
        assert!(
            !id_prefix.contains(&id_suffix),
            "idPrefix {id_prefix} cannot contain idSuffix {id_suffix}"
        );

        let password_encoder_for_encode = id_to_password_encoder
            .get(&id_for_encode)
            .cloned()
            .unwrap_or_else(|| {
                panic!("idForEncode {id_for_encode} is not found in idToPasswordEncoder")
            });

        for id in id_to_password_encoder.keys() {
            if !id_prefix.is_empty() && id.contains(&id_prefix) {
                panic!("id {id} cannot contain {id_prefix}");
            }
            if id.contains(&id_suffix) {
                panic!("id {id} cannot contain {id_suffix}");
            }
        }

        Self {
            id_prefix,
            id_suffix,
            id_for_encode,
            password_encoder_for_encode,
            id_to_password_encoder,
            default_password_encoder_for_matches: None,
        }
    }

    /// Returns the configured id prefix.
    pub fn id_prefix(&self) -> &str {
        &self.id_prefix
    }

    /// Returns the configured id suffix.
    pub fn id_suffix(&self) -> &str {
        &self.id_suffix
    }

    /// Returns the id used for encoding.
    pub fn id_for_encode(&self) -> &str {
        &self.id_for_encode
    }

    /// Sets the [`PasswordEncoder`] to delegate to for [`PasswordEncoder::matches`] when
    /// the id is not mapped to a [`PasswordEncoder`].
    ///
    /// By default, an unmapped id is reported as an error and matching returns `false`.
    ///
    /// The `encoded_password` provided will be the full password passed in, including the
    /// id portion. For example, if the password `{notmapped}foobar` were used, the "id"
    /// would be "notmapped" and the encoded password passed into the
    /// [`PasswordEncoder`] would be `{notmapped}foobar`.
    ///
    /// # Panics
    ///
    /// Panics if `default_password_encoder_for_matches` is `None`.
    pub fn set_default_password_encoder_for_matches(
        &mut self,
        default_password_encoder_for_matches: Arc<dyn PasswordEncoder>,
    ) {
        self.default_password_encoder_for_matches = Some(default_password_encoder_for_matches);
    }

    /// Extracts the id from a password prefixed with `{id}`.
    ///
    /// Returns `None` when the password does not start with the id prefix or when the id
    /// suffix is missing.
    fn extract_id<'a>(&self, encoded_password: &'a str) -> Option<&'a str> {
        let start = encoded_password.find(&self.id_prefix)?;
        if start != 0 {
            return None;
        }

        let end = encoded_password[start..].find(&self.id_suffix)? + start;
        Some(&encoded_password[start + self.id_prefix.len()..end])
    }

    /// Extracts the encoded password that follows the id suffix.
    fn extract_encoded_password<'a>(&self, encoded_password: &'a str) -> &'a str {
        match encoded_password.find(&self.id_suffix) {
            Some(index) => &encoded_password[index + self.id_suffix.len()..],
            None => encoded_password,
        }
    }

    /// Handles a password whose id is not mapped and for which no default encoder is set.
    ///
    /// The Java implementation throws an `IllegalArgumentException`; because
    /// [`PasswordEncoder::matches`] returns `bool`, the corresponding diagnostics are
    /// logged and `false` is returned instead.
    fn unmapped_id_matches(&self, encoded_password: &str) -> bool {
        let id = self.extract_id(encoded_password);
        if let Some(id) = id {
            if !id.trim().is_empty() {
                error!(
                    "{}",
                    NO_PASSWORD_ENCODER_MAPPED.replace("%s", id)
                );
                return false;
            }
        }

        if !encoded_password.trim().is_empty() {
            let start = encoded_password.find(&self.id_prefix);
            let end = match start {
                Some(start) => encoded_password[start..]
                    .find(&self.id_suffix)
                    .map(|index| index + start),
                None => encoded_password.find(&self.id_suffix),
            };
            if start.is_none() && end.is_none() {
                error!("{}", NO_PASSWORD_ENCODER_PREFIX);
                return false;
            }
        }

        error!(
            "{}",
            MALFORMED_PASSWORD_ENCODER_PREFIX
                .replace("%s", &self.id_prefix)
                .replace("%s", &self.id_suffix)
        );

        false
    }
}

impl PasswordEncoder for DelegatingPasswordEncoder {
    /// Encodes the raw password using the configured encoder, prefixing the result with
    /// the id.
    fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError> {
        let Some(raw_password) = raw_password else {
            return Ok(None);
        };

        let encoded_password = self
            .password_encoder_for_encode
            .encode(Some(raw_password))?;

        Ok(encoded_password.map(|encoded_password| {
            format!(
                "{}{}{}{}",
                self.id_prefix, self.id_for_encode, self.id_suffix, encoded_password
            )
        }))
    }

    /// Verifies the password using the encoder mapped to the password id.
    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        let delegate = self
            .extract_id(encoded_password)
            .and_then(|id| self.id_to_password_encoder.get(id));

        match delegate {
            Some(delegate) => {
                let encoded_password = self.extract_encoded_password(encoded_password);
                delegate.matches(raw_password, encoded_password)
            }
            None => match self.default_password_encoder_for_matches.as_ref() {
                Some(default_encoder) => default_encoder.matches(raw_password, encoded_password),
                None => self.unmapped_id_matches(encoded_password),
            },
        }
    }

    /// Returns `true` if the password should be re-encoded using the configured encoder.
    fn upgrade_encoding(&self, encoded_password: &str) -> Result<bool, BoxError> {
        let id = self.extract_id(encoded_password);
        if !id
            .map(|id| id.eq_ignore_ascii_case(&self.id_for_encode))
            .unwrap_or(false)
        {
            return Ok(true);
        }

        let encoded_password = self.extract_encoded_password(encoded_password);
        self.password_encoder_for_encode
            .upgrade_encoding(encoded_password)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::error::BoxError;

    use super::{DelegatingPasswordEncoder, PasswordEncoder};

    /// A password encoder that records the raw password in the encoded value.
    #[derive(Clone)]
    struct MockPasswordEncoder {
        encoded_prefix: String,
        upgrade: bool,
    }

    impl MockPasswordEncoder {
        fn new(encoded_prefix: &str, upgrade: bool) -> Self {
            Self {
                encoded_prefix: encoded_prefix.to_string(),
                upgrade,
            }
        }
    }

    impl PasswordEncoder for MockPasswordEncoder {
        fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError> {
            Ok(raw_password.map(|raw| format!("{}{}", self.encoded_prefix, raw)))
        }

        fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
            encoded_password == format!("{}{}", self.encoded_prefix, raw_password)
        }

        fn upgrade_encoding(&self, _encoded_password: &str) -> Result<bool, BoxError> {
            Ok(self.upgrade)
        }
    }

    /// A password encoder that matches every password.
    #[derive(Clone)]
    struct AlwaysMatchPasswordEncoder;

    impl PasswordEncoder for AlwaysMatchPasswordEncoder {
        fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError> {
            Ok(raw_password.map(str::to_string))
        }

        fn matches(&self, _raw_password: &str, _encoded_password: &str) -> bool {
            true
        }
    }

    fn encoder() -> DelegatingPasswordEncoder {
        let mut encoders = std::collections::HashMap::new();
        encoders.insert(
            "bcrypt".to_string(),
            Arc::new(MockPasswordEncoder::new("$2a$", false)) as Arc<dyn PasswordEncoder>,
        );
        DelegatingPasswordEncoder::new("bcrypt", encoders)
    }

    #[test]
    fn encodes_with_the_configured_id() {
        let encoder = encoder();

        assert_eq!(
            encoder.encode(Some("password")).unwrap(),
            Some("{bcrypt}$2a$password".to_string())
        );
    }

    #[test]
    fn encoding_none_returns_none() {
        assert_eq!(encoder().encode(None).unwrap(), None);
    }

    #[test]
    fn matches_delegates_to_the_mapped_encoder() {
        let encoder = encoder();

        assert!(encoder.matches("password", "{bcrypt}$2a$password"));
        assert!(!encoder.matches("password", "{bcrypt}$2a$other"));
    }

    #[test]
    fn matches_uses_the_default_encoder_for_unmapped_ids() {
        let mut encoder = encoder();
        encoder.set_default_password_encoder_for_matches(Arc::new(AlwaysMatchPasswordEncoder));

        assert!(encoder.matches("password", "{unmapped}whatever"));
    }

    #[test]
    fn matches_returns_false_for_unmapped_ids_without_default() {
        assert!(!encoder().matches("password", "{unmapped}whatever"));
    }

    #[test]
    fn upgrade_encoding_is_true_for_a_different_id() {
        assert!(encoder().upgrade_encoding("{other}value").unwrap());
    }

    #[test]
    fn upgrade_encoding_delegates_to_the_mapped_encoder() {
        let mut encoders = std::collections::HashMap::new();
        encoders.insert(
            "bcrypt".to_string(),
            Arc::new(MockPasswordEncoder::new("", true)) as Arc<dyn PasswordEncoder>,
        );
        let encoder = DelegatingPasswordEncoder::new("bcrypt", encoders);

        assert!(encoder.upgrade_encoding("{bcrypt}value").unwrap());
    }

    #[test]
    fn supports_custom_id_prefix_and_suffix() {
        let mut encoders = std::collections::HashMap::new();
        encoders.insert(
            "bcrypt".to_string(),
            Arc::new(MockPasswordEncoder::new("$2a$", false)) as Arc<dyn PasswordEncoder>,
        );
        let encoder =
            DelegatingPasswordEncoder::with_id_prefix_and_suffix("bcrypt", encoders, "[", "]");

        assert_eq!(
            encoder.encode(Some("password")).unwrap(),
            Some("[bcrypt]$2a$password".to_string())
        );
        assert!(encoder.matches("password", "[bcrypt]$2a$password"));
    }

    #[test]
    #[should_panic(expected = "idForEncode")]
    fn panics_when_id_for_encode_is_missing() {
        let encoders = std::collections::HashMap::new();
        let _ = DelegatingPasswordEncoder::new("bcrypt", encoders);
    }

    #[test]
    #[should_panic(expected = "suffix cannot be empty")]
    fn panics_when_suffix_is_empty() {
        let mut encoders = std::collections::HashMap::new();
        encoders.insert(
            "bcrypt".to_string(),
            Arc::new(MockPasswordEncoder::new("$2a$", false)) as Arc<dyn PasswordEncoder>,
        );
        let _ = DelegatingPasswordEncoder::with_id_prefix_and_suffix("bcrypt", encoders, "{", "");
    }
}
