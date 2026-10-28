//! Security, authentication, and authorization for Next Web.
//!
//! The crate carries two complete trees: `v1`, the Spring Security style
//! translation, and `v2`, the Apache Shiro style translation. Each tree names
//! modules like `core` and `web` at the root of the crate, so only one of them
//! can be compiled into an application; the feature that selects the tree also
//! decides which one a build gets.
//!
//! The `v2` tree follows Apache Shiro in its layering as well: `feature = "v2"`
//! builds the core tree — authentication, authorization, realms, sessions and
//! caches — without any dependency on HTTP, and `feature = "v2-web"` adds the web
//! integration (the filter chain, the session cookie, the HTTP filters) on top of
//! it. An application that only authenticates a token, or only asks whether a
//! subject holds a role, builds with `--no-default-features --features v2`.

#[cfg(all(feature = "v1", feature = "v2"))]
compile_error!(
    "`v1` and `v2` are two complete trees and cannot be compiled together: \
     enable only one of them, for example `--no-default-features --features v1`. \
     The protocol features (`saml2`, `webauthn`, `oauth2-*`) extend `v1`, so they \
     are built with `--no-default-features --features v1,saml2`."
);

#[cfg(feature = "v2")]
mod v2;
#[cfg(feature = "v2")]
pub use v2::*;

#[cfg(feature = "v1")]
mod v1;
#[cfg(feature = "v1")]
pub use v1::*;
