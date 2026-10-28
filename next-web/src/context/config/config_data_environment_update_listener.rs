//! Listener that is notified while config data is applied to an environment.

use crate::context::config::config_data_location::ConfigDataLocation;
use crate::context::config::config_data_profiles::ConfigDataProfiles;

/// Listener that can be used to track the updates that config data makes to an
/// environment.
///
/// Property sources are owned by the environment once they have been applied,
/// so the listener is told the name of the added source instead of the source
/// itself.
pub trait ConfigDataEnvironmentUpdateListener {
    /// Called when a property source is added to the environment.
    ///
    /// # Arguments
    ///
    /// * `property_source` - The name of the added property source.
    /// * `location` - The location the property source was loaded from, when it
    ///   was loaded from a location.
    #[allow(unused_variables)]
    fn on_property_source_added(
        &self,
        property_source: &str,
        location: Option<&ConfigDataLocation>,
    ) {
    }

    /// Called when the active and default profiles of the environment are set.
    ///
    /// # Arguments
    ///
    /// * `profiles` - The profiles that were applied.
    #[allow(unused_variables)]
    fn on_set_profiles(&self, profiles: &ConfigDataProfiles) {}
}

/// A [`ConfigDataEnvironmentUpdateListener`] that does nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpConfigDataEnvironmentUpdateListener;

impl ConfigDataEnvironmentUpdateListener for NoOpConfigDataEnvironmentUpdateListener {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_no_op_listener_ignores_every_update() {
        let listener = NoOpConfigDataEnvironmentUpdateListener;

        listener.on_property_source_added("test", None);
        listener.on_set_profiles(&ConfigDataProfiles::default());
    }
}
