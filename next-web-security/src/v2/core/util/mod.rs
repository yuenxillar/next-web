pub mod ant_path_matcher;
pub mod destroyable;
pub mod map_context;
pub mod nameable;
pub mod object;
pub mod pattern_matcher;
pub mod redirect_view;
// The helpers that read a servlet-style request and response. They belong to
// the web integration, not to the core tree.
#[cfg(feature = "web")]
pub mod web;
