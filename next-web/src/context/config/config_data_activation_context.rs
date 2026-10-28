//! The activation context used while config data is processed.

use crate::context::config::config_data_profiles::ConfigDataProfiles;

/// The context that determines which parts of the config data are active.
///
/// The profiles of the context are unknown while the first locations are
/// processed, and are filled in once the properties that declare them have been
/// loaded from the config data itself.
#[derive(Debug, Clone, Default)]
pub struct ConfigDataActivationContext {
    profiles: Option<ConfigDataProfiles>,
}

impl ConfigDataActivationContext {
    /// Creates a new context with the given profiles, if they are known yet.
    ///
    /// # Arguments
    ///
    /// * `profiles` - The profiles in use, or `None` while they are unknown.
    pub(crate) fn new(profiles: Option<ConfigDataProfiles>) -> Self {
        Self { profiles }
    }

    /// Returns a copy of this context with the given profiles.
    ///
    /// # Arguments
    ///
    /// * `profiles` - The profiles in use.
    pub(crate) fn with_profiles(&self, profiles: ConfigDataProfiles) -> Self {
        Self {
            profiles: Some(profiles),
        }
    }

    /// Returns the profiles in use, if they are known.
    pub fn get_profiles(&self) -> Option<&ConfigDataProfiles> {
        self.profiles.as_ref()
    }

    /// Returns the active profiles, or an empty slice while they are unknown.
    pub(crate) fn active_profiles(&self) -> &[String] {
        self.profiles
            .as_ref()
            .map(ConfigDataProfiles::active)
            .unwrap_or(&[])
    }

    /// Returns whether one of the given profile expressions matches.
    ///
    /// Nothing is active while the profiles are unknown.
    ///
    /// # Arguments
    ///
    /// * `profile_expressions` - The expressions to match.
    pub(crate) fn accepts(&self, profile_expressions: &[&str]) -> bool {
        self.profiles
            .as_ref()
            .map(|profiles| profiles.accepts(profile_expressions))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_active_without_profiles() {
        let context = ConfigDataActivationContext::new(None);

        assert!(context.get_profiles().is_none());
        assert!(context.active_profiles().is_empty());
        assert!(!context.accepts(&["dev"]));
    }

    #[test]
    fn evaluates_expressions_against_the_profiles() {
        let profiles = ConfigDataProfiles::new(vec!["dev".to_owned()], Vec::new());
        let context = ConfigDataActivationContext::new(None).with_profiles(profiles);

        assert_eq!(context.active_profiles(), &["dev".to_owned()]);
        assert!(context.accepts(&["dev"]));
        assert!(!context.accepts(&["prod"]));
    }
}
