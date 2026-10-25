//! Wire-level protocol shared by every lock flavour.
//!
//! This module owns everything that ends up on the socket: key naming, the Lua
//! scripts, the request builders that bind `KEYS`/`ARGV`, and the interpretation
//! of what the scripts return.

pub mod reply;

pub use reply::ScriptReply;
