use std::sync::Arc;

use crate::core::{granted_authority::GrantedAuthority, userdetails::UserDetails, Authentication};

/// Allows subclasses to fine-tune the authorities granted to the target user
/// during a switch user operation.
///
/// @see SwitchUserFilter#setSwitchUserAuthorityChanger
pub trait SwitchUserAuthorityChanger
where
    Self: Send + Sync,
{
    /// Modifies the list of granted authorities that will be assigned to
    /// the target user during a switch.
    ///
    /// # Parameters
    /// * `target_user`              - The user being switched to
    /// * `current_authentication`   - The authentication of the user requesting the switch
    /// * `authorities_to_be_granted` - The initial list of authorities from the target user
    ///
    /// # Returns
    /// The modified list of authorities to grant.
    fn modify_granted_authorities(
        &self,
        target_user: &dyn UserDetails,
        current_authentication: &dyn Authentication,
        authorities_to_be_granted: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Vec<Arc<dyn GrantedAuthority>>;
}
