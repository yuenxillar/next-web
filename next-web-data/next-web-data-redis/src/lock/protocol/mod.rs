//! Wire-level protocol shared by every lock flavour.
//!
//! This module owns everything that ends up on the socket: key naming, the Lua
//! scripts, the request builders that bind `KEYS`/`ARGV`, and the interpretation
//! of what the scripts return.

pub mod commands;
pub mod keys;
pub mod reply;
pub mod scripts;

pub use commands::LockRequest;
pub use keys::{channel_name, prefixed_name};
pub use reply::ScriptReply;
pub use scripts::{LockScript, ScriptArg, UNLOCK_MESSAGE};
