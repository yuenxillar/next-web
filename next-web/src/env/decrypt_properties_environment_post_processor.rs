//! An [`EnvironmentPostProcessor`] that decrypts the encrypted values of the
//! configuration.

use next_web_core::{
    constants::application_constants::NEXT_DECRYPT_PASSWORD,
    env::{BoxedPropertySource, ConfigurableEnvironment, PropertySource, PropertySourceValue},
    util::indexmap::IndexMap,
    Ordered,
};
use tracing::{trace, warn};

use super::PropertyDecryptor;
use crate::context::properties::source::ConfigurationPropertySources;
use crate::util::InventoryHelper;
use crate::EnvironmentPostProcessor;

/// The property that holds the password of the encrypted values.
pub const DECRYPT_PASSWORD_PROPERTY: &str = "next.decrypt.password";

/// The property that holds the password of the encrypted values, as it is
/// declared by the `--decrypt-password` argument of the application.
pub const DECRYPT_PASSWORD_ARGUMENT: &str = "decrypt-password";

/// An [`EnvironmentPostProcessor`] that replaces the values of the properties
/// that are marked as encrypted with their plaintext.
///
/// A property is decrypted when its value starts with the prefix of a
/// [`PropertyDecryptor`], which is the
/// [`SECURE_PROPERTY_PREFIX`](super::SECURE_PROPERTY_PREFIX) (`nsp:`) for the
/// decryptor of the framework. The decrypted values are installed in the
/// environment, so the properties of the configuration files are read by the
/// rest of the application as if they had been written in plaintext.
///
/// The password is taken from the [`DECRYPT_PASSWORD_PROPERTY`] property, from
/// the [`DECRYPT_PASSWORD_ARGUMENT`] property, or from the
/// [`NEXT_DECRYPT_PASSWORD`] environment variable. Encrypted values are left
/// unchanged, with a warning, when no password is configured.
///
/// # Extension point
///
/// A decryptor that an application registers with
/// [`submit_decryptor!`](crate::submit_decryptor) is applied to every property
/// whose value starts with the prefix it declares, so an application adds its
/// own algorithm without changing the framework.
pub struct DecryptPropertiesEnvironmentPostProcessor {
    password: Option<String>,
    decryptors: Vec<Box<dyn PropertyDecryptor>>,
}

impl DecryptPropertiesEnvironmentPostProcessor {
    /// The order of the processor.
    ///
    /// The processor runs just after the config data has been loaded, so that
    /// the values of the configuration files are decrypted before the
    /// application reads them.
    pub const ORDER: i32 = i32::MIN + 11;

    /// Creates a processor that decrypts with the given decryptors.
    ///
    /// # Arguments
    ///
    /// * `decryptors` - The decryptors to apply, in the order they are tried.
    pub fn new(decryptors: Vec<Box<dyn PropertyDecryptor>>) -> Self {
        Self {
            password: None,
            decryptors,
        }
    }

    /// Sets the password of the encrypted values, which takes precedence over
    /// the password that is configured through the environment.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to decrypt with.
    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = Some(password.into());
    }

    /// Sets the password of the encrypted values, and returns the processor.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to decrypt with.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.set_password(password);
        self
    }

    /// Adds a decryptor to the decryptors of this processor.
    ///
    /// The decryptors of the processor are tried before the decryptors that
    /// were registered with [`submit_decryptor!`](crate::submit_decryptor).
    ///
    /// # Arguments
    ///
    /// * `decryptor` - The decryptor to add.
    pub fn add_decryptor(&mut self, decryptor: impl PropertyDecryptor) {
        self.decryptors.push(Box::new(decryptor));
    }

    /// Adds a decryptor to the decryptors of this processor, and returns the
    /// processor.
    ///
    /// # Arguments
    ///
    /// * `decryptor` - The decryptor to add.
    pub fn with_decryptor(mut self, decryptor: impl PropertyDecryptor) -> Self {
        self.add_decryptor(decryptor);
        self
    }

    /// Replaces the decryptors of this processor.
    ///
    /// # Arguments
    ///
    /// * `decryptors` - The decryptors to apply, in the order they are tried.
    pub fn set_decryptors(&mut self, decryptors: Vec<Box<dyn PropertyDecryptor>>) {
        self.decryptors = decryptors;
    }

    /// Returns the decryptors of this processor.
    pub fn decryptors(&self) -> &[Box<dyn PropertyDecryptor>] {
        &self.decryptors
    }

    /// Returns the decryptors of this processor, followed by the decryptors
    /// that were registered with [`submit_decryptor!`](crate::submit_decryptor).
    fn available_decryptors(&self) -> Vec<&dyn PropertyDecryptor> {
        let mut decryptors: Vec<&dyn PropertyDecryptor> = self
            .decryptors
            .iter()
            .map(|decryptor| decryptor.as_ref())
            .collect();
        decryptors.extend(
            InventoryHelper::iter::<&'static dyn PropertyDecryptor>()
                .into_iter()
                .map(|it| *it),
        );
        decryptors
    }

    /// Returns the password the values are decrypted with.
    ///
    /// The password of this processor wins, then the
    /// [`DECRYPT_PASSWORD_PROPERTY`] property, then the
    /// [`DECRYPT_PASSWORD_ARGUMENT`] property of the `--decrypt-password`
    /// argument, and finally the [`NEXT_DECRYPT_PASSWORD`] environment variable.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment that holds the password properties.
    fn resolve_password(&self, environment: &dyn ConfigurableEnvironment) -> Option<String> {
        if let Some(password) = self.password.as_ref() {
            return Some(password.clone());
        }

        environment
            .get_property(DECRYPT_PASSWORD_PROPERTY)
            .or_else(|| environment.get_property(DECRYPT_PASSWORD_ARGUMENT))
            .or_else(|| std::env::var(NEXT_DECRYPT_PASSWORD).ok())
            .filter(|password| !password.is_empty())
    }
}

impl Default for DecryptPropertiesEnvironmentPostProcessor {
    /// Creates a processor holding the decryptors of the framework.
    fn default() -> Self {
        Self::new(default_decryptors())
    }
}

impl Ordered for DecryptPropertiesEnvironmentPostProcessor {
    fn order(&self) -> i32 {
        Self::ORDER
    }
}

impl EnvironmentPostProcessor for DecryptPropertiesEnvironmentPostProcessor {
    /// Decrypts the values of the environment that are marked as encrypted.
    ///
    /// # Panics
    ///
    /// Panics when a value that is marked as encrypted cannot be decrypted, so
    /// that the application never continues with a value it cannot read.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to decrypt the values of.
    fn post_process_environment(&mut self, environment: &mut dyn ConfigurableEnvironment) {
        let decryptors = self.available_decryptors();
        if decryptors.is_empty() {
            return;
        }

        let password = self.resolve_password(environment);
        let mut undecrypted: Vec<String> = Vec::new();

        for name in decryptable_sources(environment) {
            let values = decrypt_source_values(
                environment,
                &name,
                &decryptors,
                password.as_deref(),
                &mut undecrypted,
            );
            if values.is_empty() {
                continue;
            }

            trace!("Decrypting the values of the property source '{name}'");
            install_decrypted_values(environment, &name, values);
        }

        if !undecrypted.is_empty() {
            let count = undecrypted.len();
            warn!(
                "{count} encrypted {} {} not decrypted: no password is configured. \
                 Set the '{}' property or the '{}' environment variable. Encrypted properties: {}",
                if count == 1 { "property" } else { "properties" },
                if count == 1 { "was" } else { "were" },
                DECRYPT_PASSWORD_PROPERTY,
                NEXT_DECRYPT_PASSWORD,
                undecrypted.join(", ")
            );
        }
    }
}

/// Returns the decryptors the framework applies by default.
fn default_decryptors() -> Vec<Box<dyn PropertyDecryptor>> {
    #[cfg(feature = "decrypt-properties")]
    {
        vec![Box::new(
            crate::env::property_decryptor::AesPropertyDecryptor,
        )]
    }

    #[cfg(not(feature = "decrypt-properties"))]
    {
        Vec::new()
    }
}

/// Returns the names of the property sources that may hold encrypted values.
///
/// Stub sources only reserve a position and hold no value, and the attached
/// configuration property source is an adapter over the sources that are
/// inspected here, so both are skipped.
///
/// # Arguments
///
/// * `environment` - The environment to inspect.
fn decryptable_sources(environment: &dyn ConfigurableEnvironment) -> Vec<String> {
    environment
        .property_sources_ref()
        .iter()
        .filter(|source| !source.is_stub())
        .filter(|source| {
            !ConfigurationPropertySources::is_attached_configuration_property_source(*source)
        })
        .map(|source| source.name().to_owned())
        .collect()
}

/// Returns the decrypted values of the given property source.
///
/// The result holds an entry for every property that was decrypted, and is
/// empty when the source holds no encrypted value. Properties that are marked
/// as encrypted but have no password are reported and left unchanged.
///
/// # Arguments
///
/// * `environment` - The environment that holds the property source.
/// * `name` - The name of the property source to decrypt.
/// * `decryptors` - The decryptors to apply, in the order they are tried.
/// * `password` - The password to decrypt with, when one is configured.
/// * `undecrypted` - Collects the properties that have no password.
fn decrypt_source_values(
    environment: &dyn ConfigurableEnvironment,
    name: &str,
    decryptors: &[&dyn PropertyDecryptor],
    password: Option<&str>,
    undecrypted: &mut Vec<String>,
) -> IndexMap<String, String> {
    let Some(source) = environment.property_sources_ref().get(name) else {
        return IndexMap::new();
    };

    let mut values = IndexMap::new();

    for property_name in source.property_names() {
        let Some(value) = source.property(&property_name) else {
            continue;
        };
        let Some((decryptor, ciphertext)) = decryptor_of(decryptors, &value) else {
            continue;
        };

        match password {
            Some(password) => match decryptor.decrypt(ciphertext, password) {
                Ok(plaintext) => {
                    values.insert(property_name, plaintext);
                }
                Err(error) => panic!(
                    "The property '{property_name}' of '{name}' could not be decrypted: {error}"
                ),
            },
            None => undecrypted.push(property_name),
        }
    }

    values
}

/// Returns the decryptor that handles the given value, together with the
/// ciphertext that follows its prefix.
///
/// # Arguments
///
/// * `decryptors` - The decryptors to search, in the order they are tried.
/// * `value` - The value of a property.
fn decryptor_of<'a, 'value>(
    decryptors: &[&'a dyn PropertyDecryptor],
    value: &'value str,
) -> Option<(&'a dyn PropertyDecryptor, &'value str)> {
    decryptors.iter().find_map(|decryptor| {
        let prefix = decryptor.prefix();
        if prefix.is_empty() {
            return None;
        }

        value
            .strip_prefix(prefix)
            .map(|ciphertext| (*decryptor, ciphertext.trim()))
    })
}

/// Replaces the given property source with a source that returns the decrypted
/// values and delegates every other property to the original source.
///
/// The replacement keeps the position of the original source, so the precedence
/// of the properties does not change.
///
/// # Arguments
///
/// * `environment` - The environment that holds the property source.
/// * `name` - The name of the property source to replace.
/// * `values` - The decrypted values of the source.
fn install_decrypted_values(
    environment: &mut dyn ConfigurableEnvironment,
    name: &str,
    values: IndexMap<String, String>,
) {
    let previous = environment
        .property_sources_ref()
        .iter()
        .take_while(|source| source.name() != name)
        .last()
        .map(|source| source.name().to_owned());

    let Some(source) = environment.property_sources().remove(name) else {
        return;
    };
    let source: BoxedPropertySource = Box::new(DecryptedPropertySource::new(source, values));

    match previous {
        Some(previous) => environment.property_sources().add_after(&previous, source),
        None => environment.property_sources().add_first(source),
    }
}

/// A [`PropertySource`] that returns the decrypted value of the properties that
/// are marked as encrypted and delegates every other property to the source it
/// wraps.
///
/// Delegating keeps the behavior of the original source: only the values that
/// were decrypted are replaced, while the position of the source, the order and
/// the names of its properties, and the special behavior of a source such as a
/// system environment source are kept.
struct DecryptedPropertySource {
    delegate: BoxedPropertySource,
    values: IndexMap<String, String>,
}

impl DecryptedPropertySource {
    /// Creates a source that returns the given values instead of the values of
    /// the source it wraps.
    ///
    /// # Arguments
    ///
    /// * `delegate` - The source that holds the encrypted values.
    /// * `values` - The decrypted values, by property name.
    fn new(delegate: BoxedPropertySource, values: IndexMap<String, String>) -> Self {
        Self { delegate, values }
    }
}

impl std::fmt::Debug for DecryptedPropertySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecryptedPropertySource")
            .field("name", &self.delegate.name())
            .field("properties", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl PropertySource<PropertySourceValue> for DecryptedPropertySource {
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn property(&self, name: &str) -> Option<String> {
        match self.values.get(name) {
            Some(value) => Some(value.clone()),
            None => self.delegate.property(name),
        }
    }

    fn contains_property(&self, name: &str) -> bool {
        self.values.contains_key(name) || self.delegate.contains_property(name)
    }

    fn property_names(&self) -> Vec<String> {
        self.delegate.property_names()
    }

    fn is_stub(&self) -> bool {
        self.delegate.is_stub()
    }

    fn source(&self) -> &PropertySourceValue {
        self.delegate.source()
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::{
        env::{
            MapPropertySource, PropertyResolver, StandardEnvironment,
            SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME,
        },
        error::BoxError,
    };

    use super::*;

    /// A property source holding the given properties.
    fn source(name: &str, properties: &[(&str, &str)]) -> BoxedPropertySource {
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        Box::new(MapPropertySource::new(name.to_owned(), properties))
    }

    /// An environment holding the given property sources.
    ///
    /// The property sources of the test process are removed, so that the tests
    /// do not depend on the environment variables of the process.
    fn environment(sources: &[(&str, &[(&str, &str)])]) -> StandardEnvironment {
        let mut environment = StandardEnvironment::new();
        let _ = environment
            .property_sources()
            .remove(SYSTEM_ENVIRONMENT_PROPERTY_SOURCE_NAME);

        for (name, properties) in sources {
            environment
                .property_sources()
                .add_last(source(name, properties));
        }

        environment
    }

    /// Returns the names of the property sources of the environment.
    fn source_names(environment: &StandardEnvironment) -> Vec<String> {
        environment
            .property_sources_ref()
            .iter()
            .map(|source| source.name().to_owned())
            .collect()
    }

    /// A decryptor that reverses the ciphertext, registered with the macro an
    /// application uses, so that the extension point is exercised as a user of
    /// the framework would.
    struct ReverseDecryptor;

    impl PropertyDecryptor for ReverseDecryptor {
        fn prefix(&self) -> &str {
            "reverse:"
        }

        fn decrypt(&self, ciphertext: &str, _password: &str) -> Result<String, BoxError> {
            Ok(ciphertext.chars().rev().collect())
        }
    }

    crate::submit_decryptor!(ReverseDecryptor);

    #[cfg(feature = "decrypt-properties")]
    #[test]
    fn decrypts_the_values_marked_as_encrypted() {
        let ciphertext = crate::util::aes::encrypt("s3cr3t", "password").unwrap();
        let encrypted = format!(
            "{}{ciphertext}",
            crate::env::property_decryptor::SECURE_PROPERTY_PREFIX
        );
        let mut environment = environment(&[(
            "application.yaml",
            &[
                ("next.data.redis.password", encrypted.as_str()),
                ("next.data.redis.host", "192.168.1.19"),
            ],
        )]);

        DecryptPropertiesEnvironmentPostProcessor::default()
            .with_password("password")
            .post_process_environment(&mut environment);

        assert_eq!(
            environment.get_property("next.data.redis.password"),
            Some("s3cr3t".to_owned())
        );
        assert_eq!(
            environment.get_property("next.data.redis.host"),
            Some("192.168.1.19".to_owned())
        );
    }

    #[test]
    fn applies_the_decryptors_that_were_registered_with_the_macro() {
        let mut environment =
            environment(&[("vault", &[("next.data.redis.password", "reverse:abc")])]);

        DecryptPropertiesEnvironmentPostProcessor::default()
            .with_password("password")
            .post_process_environment(&mut environment);

        assert_eq!(
            environment.get_property("next.data.redis.password"),
            Some("cba".to_owned())
        );
    }

    #[test]
    fn keeps_the_position_of_the_source_and_the_values_that_are_not_encrypted() {
        let mut environment = environment(&[
            ("first", &[("next.application.name", "first")]),
            (
                "second",
                &[
                    ("next.data.redis.password", "reverse:abc"),
                    ("next.data.redis.host", "localhost"),
                ],
            ),
            ("third", &[("next.application.version", "1")]),
        ]);

        DecryptPropertiesEnvironmentPostProcessor::default()
            .with_password("password")
            .post_process_environment(&mut environment);

        assert_eq!(source_names(&environment), vec!["first", "second", "third"]);
        assert_eq!(
            environment.get_property("next.data.redis.password"),
            Some("cba".to_owned())
        );
        assert_eq!(
            environment.get_property("next.data.redis.host"),
            Some("localhost".to_owned())
        );
        assert_eq!(
            environment.get_property("next.application.name"),
            Some("first".to_owned())
        );
    }

    #[test]
    fn takes_the_password_from_the_environment() {
        let mut environment = environment(&[
            (
                "application.yaml",
                &[(DECRYPT_PASSWORD_PROPERTY, "password")],
            ),
            ("vault", &[("next.data.redis.password", "reverse:abc")]),
        ]);

        DecryptPropertiesEnvironmentPostProcessor::default()
            .post_process_environment(&mut environment);

        assert_eq!(
            environment.get_property("next.data.redis.password"),
            Some("cba".to_owned())
        );
    }

    #[test]
    fn takes_the_password_of_the_decrypt_password_argument() {
        let mut environment = environment(&[
            (
                "commandLineArgs",
                &[(DECRYPT_PASSWORD_ARGUMENT, "password")],
            ),
            ("vault", &[("next.data.redis.password", "reverse:abc")]),
        ]);

        DecryptPropertiesEnvironmentPostProcessor::default()
            .post_process_environment(&mut environment);

        assert_eq!(
            environment.get_property("next.data.redis.password"),
            Some("cba".to_owned())
        );
    }

    #[test]
    fn leaves_the_encrypted_values_without_a_password() {
        let environment = environment(&[("vault", &[("next.data.redis.password", "reverse:abc")])]);
        let decryptors: Vec<&dyn PropertyDecryptor> = vec![&ReverseDecryptor];
        let mut undecrypted: Vec<String> = Vec::new();

        let values =
            decrypt_source_values(&environment, "vault", &decryptors, None, &mut undecrypted);

        assert!(values.is_empty());
        assert_eq!(undecrypted, vec!["next.data.redis.password".to_owned()]);
    }

    #[test]
    #[should_panic(expected = "could not be decrypted")]
    fn fails_when_a_value_cannot_be_decrypted() {
        /// A decryptor that never decrypts.
        struct FailingDecryptor;

        impl PropertyDecryptor for FailingDecryptor {
            fn prefix(&self) -> &str {
                "failing:"
            }

            fn decrypt(&self, ciphertext: &str, _password: &str) -> Result<String, BoxError> {
                Err(format!("cannot decrypt '{ciphertext}'").into())
            }
        }

        let mut environment =
            environment(&[("vault", &[("next.data.redis.password", "failing:abc")])]);

        DecryptPropertiesEnvironmentPostProcessor::default()
            .with_password("password")
            .with_decryptor(FailingDecryptor)
            .post_process_environment(&mut environment);
    }
}
