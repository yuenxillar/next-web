//! The profiles that are in use while config data is processed.

use next_web_core::env::profiles_of;
use tracing::warn;

/// The property that declares the profiles that are active.
pub(crate) const ACTIVE_PROFILE_PROPERTY: &str = "next.profiles.active";

/// The property that declares the profiles that are active by default.
pub(crate) const DEFAULT_PROFILE_PROPERTY: &str = "next.profiles.default";

/// The property that declares profiles to activate in addition to the active
/// profiles.
pub(crate) const INCLUDE_PROFILES: &str = "next.profiles.include";

/// The profiles that are in use while config data is processed.
///
/// The profiles hold the names of the profiles that are active and of the
/// profiles that are used to evaluate profile expressions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigDataProfiles {
    active: Vec<String>,
    default: Vec<String>,
}

impl ConfigDataProfiles {
    /// Creates profiles for the given active and default profiles.
    ///
    /// Empty names are dropped and duplicate names are only kept once, in the
    /// order in which they were given. Profiles that are listed later therefore
    /// keep their higher precedence.
    ///
    /// # Arguments
    ///
    /// * `active` - The active profiles, in ascending precedence order.
    /// * `default` - The profiles that are active when none is active.
    pub(crate) fn new(
        active: impl IntoIterator<Item = String>,
        default: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            active: dedupe(active),
            default: dedupe(default),
        }
    }

    /// Returns the active profiles, in ascending precedence order.
    pub fn active(&self) -> &[String] {
        &self.active
    }

    /// Returns the profiles that are active when none is active.
    pub fn default_profiles(&self) -> &[String] {
        &self.default
    }

    /// Returns whether the given profile is active.
    ///
    /// # Arguments
    ///
    /// * `profile` - The name of the profile.
    pub fn is_active(&self, profile: &str) -> bool {
        self.active.iter().any(|active| active == profile)
    }

    /// Returns whether one of the given profile expressions matches the active
    /// profiles.
    ///
    /// The expressions support the `!`, `&` and `|` operators, in the same way
    /// as profile expressions in the environment.
    ///
    /// # Arguments
    ///
    /// * `profile_expressions` - The expressions to match.
    pub fn accepts(&self, profile_expressions: &[&str]) -> bool {
        let expressions: Vec<&str> = profile_expressions
            .iter()
            .map(|expression| expression.trim())
            .filter(|expression| !expression.is_empty())
            .collect();
        if expressions.is_empty() {
            return false;
        }

        match profiles_of(&expressions) {
            Ok(profiles) => profiles.matches(&|profile: &str| self.is_active(profile)),
            Err(error) => {
                warn!("Unable to parse the profile expression {expressions:?}: {error}");
                false
            }
        }
    }
}

/// Splits a comma separated profile property value into profiles.
///
/// # Arguments
///
/// * `value` - The value of a profile property.
pub(crate) fn profiles_from_value(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Removes empty and duplicate profile names.
fn dedupe(profiles: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();

    for profile in profiles {
        let profile = profile.trim();
        if profile.is_empty() || result.iter().any(|existing| existing == profile) {
            continue;
        }
        result.push(profile.to_owned());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates profiles from string slices.
    fn profiles(active: &[&str], default: &[&str]) -> ConfigDataProfiles {
        ConfigDataProfiles::new(
            active.iter().map(|profile| (*profile).to_owned()),
            default.iter().map(|profile| (*profile).to_owned()),
        )
    }

    #[test]
    fn keeps_profiles_once_and_in_order() {
        let profiles = profiles(&["dev", " ", "cloud", "dev"], &["default"]);

        assert_eq!(profiles.active(), &["dev".to_owned(), "cloud".to_owned()]);
        assert_eq!(profiles.default_profiles(), &["default".to_owned()]);
        assert!(profiles.is_active("cloud"));
        assert!(!profiles.is_active("prod"));
    }

    #[test]
    fn evaluates_profile_expressions() {
        let profiles = profiles(&["dev", "cloud"], &[]);

        assert!(profiles.accepts(&["dev"]));
        assert!(profiles.accepts(&["dev & cloud"]));
        assert!(profiles.accepts(&["prod", "cloud"]));
        assert!(!profiles.accepts(&["dev & prod"]));
        assert!(!profiles.accepts(&["!dev"]));
        assert!(!profiles.accepts(&[]));
        assert!(!profiles.accepts(&["  "]));
    }

    #[test]
    fn rejects_invalid_profile_expressions() {
        let profiles = profiles(&["dev"], &[]);

        assert!(!profiles.accepts(&["dev & | cloud"]));
    }

    #[test]
    fn splits_profile_values() {
        assert_eq!(
            profiles_from_value(" dev , cloud ,, "),
            vec!["dev".to_owned(), "cloud".to_owned()]
        );
        assert!(profiles_from_value("").is_empty());
    }
}
