use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{granted_authority::GrantedAuthority, Authentication};

/// A `GrantedAuthority` that stores the original `Authentication` for use
/// when exiting a switched user context.
///
/// This authority is added to the target user's authorities during a switch
/// operation. When the user later requests to exit the switch, the original
/// `Authentication` is retrieved from this authority.
#[derive(Clone)]
pub struct SwitchUserGrantedAuthority {
    authority_role: String,
    source: Arc<dyn Authentication>,
}

impl SwitchUserGrantedAuthority {
    /// Creates a new `SwitchUserGrantedAuthority`.
    ///
    /// # Parameters
    /// * `authority_role` - The role name to use (e.g. `ROLE_PREVIOUS_ADMINISTRATOR`)
    /// * `source`         - The original `Authentication` to restore on exit
    pub fn new(authority_role: impl Into<String>, source: Arc<dyn Authentication>) -> Self {
        Self {
            authority_role: authority_role.into(),
            source,
        }
    }

    /// Returns the source (original) `Authentication` that should be restored
    /// when the user exits the switched context.
    pub fn get_source(&self) -> &dyn Authentication {
        self.source.as_ref()
    }
}

impl GrantedAuthority for SwitchUserGrantedAuthority {
    fn authority(&self) -> Option<&str> {
        Some(self.authority_role.as_str())
    }
}
