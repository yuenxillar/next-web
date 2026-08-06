pub mod anys;
pub mod autoconfigure;
pub mod autoregister;
pub mod client;
pub mod common;
pub mod constants;
pub mod context;
pub mod cors;
pub mod error;
pub mod filter;
pub mod http;
pub mod macros;
pub mod messaging;
pub mod mime_type;
pub mod proxy;
pub mod scheduler;
pub mod server;
pub mod state;
pub mod store;
pub mod traits;
pub mod util;
pub mod web;
pub mod wrapper;

pub use self::autoregister::auto_register::*;
pub use self::context::application_context::*;
pub use arc_swap::ArcSwap;
pub use async_trait::async_trait;
pub use dyn_clone::{DynClone, clone_box, clone_trait_object};

#[cfg(feature = "http-request")]
pub extern crate headers;

pub use urlencoding;

pub type AnyObject = std::sync::Arc<dyn std::any::Any + Send + Sync>;
pub type BoxAny = std::boxed::Box<dyn std::any::Any + Send + Sync>;

mod ordered;

pub use ordered::Ordered;
