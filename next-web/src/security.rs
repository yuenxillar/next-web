//! The access-control rules a handler is guarded with.
//!
//! This is the surface the framework's own attributes are written against: the
//! `#[pre_authorize]` attribute compiles its argument into a
//! [`PreAuthorizeExpression`](access::prepost::pre_authorize::PreAuthorizeExpression)
//! and evaluates it through
//! [`evaluate`](access::prepost::pre_authorize::evaluate).
//!
//! The rules read the authorities of the request through
//! [`CurrentAuthorities`], which the crate that authenticates the requests
//! installs with [`install_current_authorities`]. An application without such a
//! crate authenticates nothing, so a rule that asks for a role or an authority
//! does not hold and only `permit_all` allows a request.

pub use next_web_core::security::access;
pub use next_web_core::security::{
    current_authorities, install_current_authorities, CurrentAuthorities,
};
