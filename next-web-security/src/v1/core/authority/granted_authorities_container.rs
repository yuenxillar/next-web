use std::sync::Arc;

use crate::core::GrantedAuthority;

/// Indicates that a object stores GrantedAuthority objects.
/// Typically used in a pre-authenticated scenario when an AuthenticationDetails instance may also be used
/// to obtain user authorities.
pub trait GrantedAuthoritiesContainer {
    fn granted_authorities(&self) -> &[Arc<dyn GrantedAuthority>];
}
