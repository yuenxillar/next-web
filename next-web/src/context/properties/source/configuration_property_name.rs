//! A configuration property name, consisting of one or more elements.

use std::fmt;

/// The separator used between the elements of a configuration property name.
const SEPARATOR: char = '.';

/// Error returned when a value is not a valid configuration property name.
///
/// This is the Rust equivalent of
/// `InvalidConfigurationPropertyNameException`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidConfigurationPropertyNameError {
    invalid_name: String,
}

impl InvalidConfigurationPropertyNameError {
    /// Creates a new error for the given invalid name.
    ///
    /// # Arguments
    ///
    /// * `invalid_name` - The value that is not a valid name.
    fn new(invalid_name: impl Into<String>) -> Self {
        Self {
            invalid_name: invalid_name.into(),
        }
    }

    /// Returns the value that is not a valid configuration property name.
    pub fn invalid_name(&self) -> &str {
        &self.invalid_name
    }
}

impl fmt::Display for InvalidConfigurationPropertyNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid configuration property name '{}'",
            self.invalid_name
        )
    }
}

impl std::error::Error for InvalidConfigurationPropertyNameError {}

/// The name of a configuration property.
///
/// A name consists of one or more elements separated by dots. This is the Rust
/// equivalent of the uniform form of Next Boot's `ConfigurationPropertyName`:
/// elements are made up of lowercase letters, digits and dashes, and each
/// element must start with a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigurationPropertyName {
    name: String,
    elements: Vec<String>,
}

impl ConfigurationPropertyName {
    /// Returns a name for the given value.
    ///
    /// # Arguments
    ///
    /// * `name` - The value to turn into a configuration property name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidConfigurationPropertyNameError`] if the value is not a
    /// valid configuration property name.
    pub fn of(name: &str) -> Result<Self, InvalidConfigurationPropertyNameError> {
        if !Self::is_valid(name) {
            return Err(InvalidConfigurationPropertyNameError::new(name));
        }

        Ok(Self {
            name: name.to_owned(),
            elements: name.split(SEPARATOR).map(ToOwned::to_owned).collect(),
        })
    }

    /// Returns whether the given value is a valid configuration property name.
    ///
    /// # Arguments
    ///
    /// * `name` - The value to check.
    pub fn is_valid(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        name.split(SEPARATOR).all(Self::is_valid_element)
    }

    /// Returns whether the given element is valid.
    ///
    /// # Arguments
    ///
    /// * `element` - The element to check.
    fn is_valid_element(element: &str) -> bool {
        let mut chars = element.chars();

        match chars.next() {
            Some(first) if first.is_ascii_lowercase() => {}
            _ => return false,
        }

        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }

    /// Returns the elements of this name, in order.
    pub fn elements(&self) -> &[String] {
        &self.elements
    }

    /// Returns the number of elements of this name.
    pub fn number_of_elements(&self) -> usize {
        self.elements.len()
    }

    /// Returns this name as a string.
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ConfigurationPropertyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_valid_names() {
        let name = ConfigurationPropertyName::of("next.datasource.url").unwrap();

        assert_eq!(name.as_str(), "next.datasource.url");
        assert_eq!(
            name.elements(),
            &[
                "next".to_owned(),
                "datasource".to_owned(),
                "url".to_owned()
            ]
        );
        assert_eq!(name.number_of_elements(), 3);
        assert_eq!(name.to_string(), "next.datasource.url");
    }

    #[test]
    fn accepts_digits_and_dashes() {
        assert!(ConfigurationPropertyName::is_valid("server.http2.enabled"));
        assert!(ConfigurationPropertyName::is_valid("a1.b-c2"));
    }

    #[test]
    fn rejects_invalid_names() {
        for name in [
            "",
            ".",
            "next..url",
            "Next.Url",
            "1next",
            "next.url_",
            "-url",
        ] {
            assert!(
                ConfigurationPropertyName::of(name).is_err(),
                "'{name}' should be invalid"
            );
            assert!(!ConfigurationPropertyName::is_valid(name));
        }
    }

    #[test]
    fn reports_the_invalid_name() {
        let error = ConfigurationPropertyName::of("Next.Url").unwrap_err();

        assert_eq!(error.invalid_name(), "Next.Url");
        assert!(error.to_string().contains("Next.Url"));
    }
}
