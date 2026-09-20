//! A configuration property, consisting of a name and a value.

use std::fmt;

use next_web_core::anys::any_value::AnyValue;

use crate::context::properties::source::ConfigurationPropertyName;

/// A single configuration property.
///
/// This is the Rust equivalent of Next Boot's `ConfigurationProperty`. The
/// Java version also carries the [`Origin`] of the value; origins are not part
/// of this conversion yet.
///
/// [`Origin`]: https://docs.next.io/next-boot/api/java/org/nextframework/boot/origin/Origin.html
#[derive(Clone)]
pub struct ConfigurationProperty {
    name: ConfigurationPropertyName,
    value: AnyValue,
}

impl ConfigurationProperty {
    /// Creates a new configuration property.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property.
    /// * `value` - The value of the property.
    pub fn new(name: ConfigurationPropertyName, value: AnyValue) -> Self {
        Self { name, value }
    }

    /// Returns the name of this property.
    pub fn name(&self) -> &ConfigurationPropertyName {
        &self.name
    }

    /// Returns the value of this property.
    pub fn get_value(&self) -> &AnyValue {
        &self.value
    }
}

impl fmt::Debug for ConfigurationProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigurationProperty")
            .field("name", &self.name.as_str())
            .field("value", &self.value.to_string())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_name_and_value() {
        let name = ConfigurationPropertyName::of("next.application.name").unwrap();
        let property = ConfigurationProperty::new(name.clone(), AnyValue::from("demo"));

        assert_eq!(property.name(), &name);
        assert_eq!(property.get_value().as_str(), Some("demo"));
    }

    #[test]
    fn formats_name_and_value() {
        let name = ConfigurationPropertyName::of("next.application.name").unwrap();
        let property = ConfigurationProperty::new(name, AnyValue::from("demo"));

        assert_eq!(
            format!("{property:?}"),
            "ConfigurationProperty { name: \"next.application.name\", value: \"demo\" }"
        );
    }
}
