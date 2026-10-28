//! A location from which config data can be loaded.

use std::fmt;

/// The prefix that marks a location as optional.
const OPTIONAL_PREFIX: &str = "optional:";

/// The prefix that marks a location as a file system path.
const FILE_PREFIX: &str = "file:";

/// A location from which config data can be loaded.
///
/// A location is made up of an optional `optional:` prefix, an optional `file:`
/// prefix and a value that refers to a resource, to a directory or to a wildcard
/// pattern:
///
/// - A location without a prefix is resolved by the resource loader, where `/`
///   is the root of the resources.
/// - A location with the `file:` prefix is resolved against the file system.
/// - A location with the `optional:` prefix may be missing without that being an
///   error.
///
/// A location only describes *where* config data can be loaded from. Turning a
/// location into the resources that are read is the job of
/// [`ConfigDataLocationResolver`](crate::context::config::ConfigDataLocationResolver).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConfigDataLocation {
    value: String,
    optional: bool,
}

impl ConfigDataLocation {
    /// Returns a location for the given value.
    ///
    /// The value is trimmed and a leading `optional:` prefix is removed and
    /// remembered. Everything else, including a `file:` prefix, is kept as-is.
    ///
    /// # Arguments
    ///
    /// * `location` - The location value, for example `optional:file:./config/`.
    pub fn of(location: impl AsRef<str>) -> Self {
        let location = location.as_ref().trim();

        match location.strip_prefix(OPTIONAL_PREFIX) {
            Some(value) => Self {
                value: value.trim().to_owned(),
                optional: true,
            },
            None => Self {
                value: location.to_owned(),
                optional: false,
            },
        }
    }

    /// Returns the value of this location, without the `optional:` prefix.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns whether this location may be missing.
    ///
    /// A missing optional location is ignored instead of being reported as an
    /// error.
    pub fn is_optional(&self) -> bool {
        self.optional
    }

    /// Returns whether this location has no value.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Returns whether this location refers to the file system.
    pub fn is_file_system(&self) -> bool {
        self.value.starts_with(FILE_PREFIX)
    }

    /// Returns whether this location contains a wildcard pattern.
    pub fn has_wildcard(&self) -> bool {
        self.value.contains('*')
    }

    /// Returns the value of this location without its `file:` prefix.
    pub fn path(&self) -> &str {
        strip_file_prefix(&self.value)
    }
}

impl fmt::Display for ConfigDataLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.optional {
            f.write_str(OPTIONAL_PREFIX)?;
        }
        f.write_str(&self.value)
    }
}

/// Removes the `file:` prefix from the given location value.
///
/// # Arguments
///
/// * `value` - The location value to strip the prefix from.
pub(crate) fn strip_file_prefix(value: &str) -> &str {
    value.strip_prefix(FILE_PREFIX).unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_locations_from_values() {
        let location = ConfigDataLocation::of("/config/");

        assert_eq!(location.value(), "/config/");
        assert_eq!(location.path(), "/config/");
        assert!(!location.is_optional());
        assert!(!location.is_file_system());
        assert!(!location.is_empty());
        assert!(!location.has_wildcard());
    }

    #[test]
    fn extracts_the_optional_prefix() {
        let location = ConfigDataLocation::of("  optional: file:./config/  ");

        assert!(location.is_optional());
        assert_eq!(location.value(), "file:./config/");
        assert_eq!(location.to_string(), "optional:file:./config/");
    }

    #[test]
    fn detects_wildcard_and_file_system_locations() {
        let location = ConfigDataLocation::of("optional:file:./config/*/");

        assert!(location.is_optional());
        assert!(location.is_file_system());
        assert!(location.has_wildcard());
        assert_eq!(location.path(), "./config/*/");
    }

    #[test]
    fn creates_locations_for_empty_values() {
        let location = ConfigDataLocation::of("   ");

        assert!(location.is_empty());
        assert_eq!(location.to_string(), "");
    }
}
