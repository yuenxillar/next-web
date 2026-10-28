//! The allocation behaviour of the event hot path is guarded structurally in
//! `tests/hot_path.rs`: a context hands out the same message instance, and a
//! listener or action dispatch shares one snapshot instead of copying the
//! collection per notification.
//!
//! A counting global allocator is not used because the workspace forbids
//! `unsafe` code, and a `GlobalAlloc` implementation requires it.
