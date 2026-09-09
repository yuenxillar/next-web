//! Lua scripts used by Redisson's lock commands.

pub(crate) const ACQUIRE_SCRIPT: &str = include_str!("scripts/acquire.lua");
pub(crate) const UNLOCK_SCRIPT: &str = include_str!("scripts/unlock.lua");
pub(crate) const FORCE_UNLOCK_SCRIPT: &str = include_str!("scripts/force_unlock.lua");
pub(crate) const RENEW_SCRIPT: &str = include_str!("scripts/renew.lua");
