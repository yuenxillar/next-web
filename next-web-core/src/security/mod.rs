//! The access-control rules an application declares on its handlers.
//!
//! The rules of this module are declared with the framework's own macros (such
//! as the `#[pre_authorize]` attribute), so the module belongs to the framework
//! and not to a crate that implements authentication: it knows the *shape* of a
//! rule and how to evaluate it, and it asks for the authorities of the current
//! request through [`CurrentAuthorities`], which whichever crate authenticates
//! the requests installs.

pub mod access;
pub mod authorities;

pub use access::prepost::pre_authorize::{
    PreAuthorizeContext, PreAuthorizeDenialHandler, AUTHORITIES_ATTRIBUTE, DENIAL_HANDLER_ATTRIBUTE,
};
pub use authorities::{current_authorities, install_current_authorities, CurrentAuthorities};
