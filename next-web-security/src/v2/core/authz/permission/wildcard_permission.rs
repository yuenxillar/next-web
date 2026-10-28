//! Shiro's wildcard permission: `printer:print,query` style permission strings.
//!
//! A permission is a series of parts separated by `:`, and each part is a set
//! of subparts separated by `,`. A `*` stands for every value: it matches any
//! part, and a permission with fewer parts implies any permission that has more
//! of them when the parts it lacks are wildcards.

use std::{collections::HashSet, fmt::Display};

use super::Permission;

/// The token that matches every part of a permission.
const WILDCARD_TOKEN: &str = "*";

/// The character that separates the parts of a permission.
const PART_DIVIDER_TOKEN: char = ':';

/// The character that separates the values of one part.
const SUBPART_DIVIDER_TOKEN: char = ',';

/// Whether the parts of a permission are matched case sensitively by default.
const DEFAULT_CASE_SENSITIVE: bool = false;

/// A permission expressed with the wildcard grammar.
///
/// ```
/// use next_web_security::core::authz::permission::wildcard_permission::WildcardPermission;
/// use next_web_security::core::authz::permission::Permission;
///
/// let granted = WildcardPermission::new("printer:*");
/// let requested = WildcardPermission::new("printer:print");
/// assert!(granted.implies(&requested));
/// assert!(!requested.implies(&granted));
/// ```
#[derive(Clone, Eq, PartialEq)]
pub struct WildcardPermission {
    parts: Vec<HashSet<String>>,
    identifier: String,
}

impl WildcardPermission {
    /// The token that matches every part of a permission.
    pub const WILDCARD_TOKEN: &'static str = WILDCARD_TOKEN;

    /// The character that separates the parts of a permission.
    pub const PART_DIVIDER_TOKEN: char = PART_DIVIDER_TOKEN;

    /// The character that separates the values of one part.
    pub const SUBPART_DIVIDER_TOKEN: char = SUBPART_DIVIDER_TOKEN;

    /// Creates a permission from a wildcard string, matched case insensitively.
    ///
    /// # Arguments
    ///
    /// * `wildcard_string` - The permission string, for example `printer:print`.
    ///
    /// # Panics
    ///
    /// Panics when the string is empty or contains a part with no value.
    pub fn new(wildcard_string: impl AsRef<str>) -> Self {
        Self::with_case_sensitive(wildcard_string, DEFAULT_CASE_SENSITIVE)
    }

    /// Creates a permission from a wildcard string.
    ///
    /// # Arguments
    ///
    /// * `wildcard_string` - The permission string, for example `printer:print`.
    /// * `case_sensitive` - Whether the parts keep their case.
    ///
    /// # Panics
    ///
    /// Panics when the string is empty or contains a part with no value.
    pub fn with_case_sensitive(wildcard_string: impl AsRef<str>, case_sensitive: bool) -> Self {
        let wildcard_string = wildcard_string.as_ref().trim().to_string();
        assert!(
            !wildcard_string.is_empty(),
            "Wildcard string cannot be null or empty. Make sure permission strings are properly formatted."
        );

        let wildcard_string = if case_sensitive {
            wildcard_string
        } else {
            wildcard_string.to_lowercase()
        };

        let parts: Vec<HashSet<String>> = wildcard_string
            .split(PART_DIVIDER_TOKEN)
            .map(|part| {
                // A value of a part is a non-empty run between the dividers, so a
                // part that is empty or made of dividers alone has no value.
                part.split(SUBPART_DIVIDER_TOKEN)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect::<HashSet<_>>()
            })
            .collect();

        for part in &parts {
            assert!(
                !part.is_empty(),
                "Wildcard string cannot contain parts with only dividers. Make sure permission strings are properly formatted."
            );
        }

        Self {
            parts,
            identifier: wildcard_string,
        }
    }

    /// Returns the parts of this permission, each as the set of its values.
    pub fn parts(&self) -> &[HashSet<String>] {
        &self.parts
    }

    /// Returns whether the given part is the wildcard of this permission.
    fn is_wildcard(part: &HashSet<String>) -> bool {
        part.contains(WILDCARD_TOKEN)
    }
}

impl Permission for WildcardPermission {
    fn implies(&self, p: &dyn Permission) -> bool {
        // Every permission identifies itself as a wildcard string, so the other
        // side of the comparison is read through its identifier. A permission of
        // another type that does not spell a wildcard string therefore implies
        // and is implied like the string it reports.
        let other = WildcardPermission::new(p.identifier());
        let other_parts = other.parts();

        let mut index = 0;
        for other_part in other_parts {
            // This permission has fewer parts than the other one: everything past
            // its own parts is implied.
            if index + 1 > self.parts.len() {
                return true;
            }

            let part = &self.parts[index];
            if !Self::is_wildcard(part) && !part.is_superset(other_part) {
                return false;
            }

            index += 1;
        }

        // This permission has more parts than the other one, so it only implies it
        // when every part the other one does not name is a wildcard.
        for part in &self.parts[index..] {
            if !Self::is_wildcard(part) {
                return false;
            }
        }

        true
    }

    fn identifier(&self) -> &str {
        &self.identifier
    }
}

impl Display for WildcardPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

#[cfg(test)]
mod tests {
    use super::WildcardPermission;
    use crate::core::authz::permission::Permission;

    fn implied(granted: &str, requested: &str) -> bool {
        WildcardPermission::new(granted).implies(&WildcardPermission::new(requested))
    }

    #[test]
    fn a_wildcard_part_implies_every_value_of_it() {
        assert!(implied("printer:*", "printer:print"));
        assert!(implied("printer:*", "printer:print,query"));
        assert!(!implied("printer:print", "printer:*"));
    }

    #[test]
    fn a_part_implies_its_own_values_only() {
        assert!(implied("printer:print", "printer:print"));
        assert!(implied("printer:print,query", "printer:query"));
        assert!(!implied("printer:print", "printer:query"));
    }

    #[test]
    fn a_permission_with_fewer_parts_is_implied_by_the_longer_one() {
        // `printer` grants everything under the printer domain.
        assert!(implied("printer", "printer:print"));
        assert!(implied("printer", "printer:print,query:color"));
    }

    #[test]
    fn a_longer_permission_implies_the_shorter_only_through_wildcards() {
        // Every part the other permission does not name has to be a wildcard.
        assert!(!implied("printer:*:color", "printer:print"));
        assert!(!implied("printer:print:color", "printer:print"));
        assert!(implied("printer:print:*", "printer:print"));
        assert!(implied("printer,user:*", "printer"));
    }

    #[test]
    fn the_wildcard_permission_implies_every_other_permission() {
        assert!(implied("*", "printer"));
        assert!(implied("*", "printer:print"));
        assert!(implied("*", "printer:print,query:color"));
        assert!(implied("*:*", "printer:print"));
    }

    #[test]
    fn permissions_are_case_insensitive_unless_asked_otherwise() {
        assert!(implied("Printer:Print", "printer:print"));

        let granted = WildcardPermission::with_case_sensitive("Printer:Print", true);
        let requested = WildcardPermission::with_case_sensitive("printer:print", true);
        assert!(!granted.implies(&requested));
    }

    #[test]
    fn the_identifier_is_the_normalized_string() {
        assert_eq!(WildcardPermission::new(" Printer:Print ").identifier(), "printer:print");
        assert_eq!(WildcardPermission::new("printer:print").to_string(), "printer:print");
    }

    #[test]
    #[should_panic(expected = "Wildcard string cannot be null or empty")]
    fn an_empty_permission_is_refused() {
        WildcardPermission::new("   ");
    }

    #[test]
    #[should_panic(expected = "parts with only dividers")]
    fn a_permission_with_a_part_of_dividers_only_is_refused() {
        // A part that is only dividers has no value at all.
        WildcardPermission::new("printer::print");
    }
}
