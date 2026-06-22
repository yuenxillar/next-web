use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{Authentication, granted_authority::GrantedAuthority};

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

#[async_trait]
impl GrantedAuthority for SwitchUserGrantedAuthority {
    async fn get_authority(&self) -> Option<String> {
        Some(self.authority_role.clone())
    }

    fn as_switch_user_source(&self) -> Option<Arc<dyn Authentication>> {
        Some(self.source.clone())
    }
}
