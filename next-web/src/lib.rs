pub mod application;
pub mod autoconfigure;
pub mod autoregister;
pub mod banner;
pub mod configurer;
pub mod crypto;
pub mod diagnostics;
pub mod error;
pub mod event;
pub mod extract;
pub mod interceptor;
pub mod macros;
pub mod manager;
pub mod stream;
pub mod util;
pub mod signal;


pub use crate::extract::required_header::header_names;

pub use axum::Router;
pub use axum::{body, error_handling, handler, http, response, routing};

pub use rand;
pub use validator as validate;

pub use next_web_core as core;

#[cfg(feature = "enable-state-machine")]
pub use next_web_state_machine as state_machine;

#[cfg(feature = "enable-web-security")]
pub use next_web_security as security;

#[cfg(feature = "enable-websocket")]
pub use next_web_websocket as ws;

#[cfg(feature = "enable-retry")]
pub use next_web_retry as retry;

#[cfg(feature = "enable-api-doc")]
pub use next_web_api_doc as api_doc;

#[cfg(feature = "enable-i18n")]
pub mod i18n;

#[cfg(feature = "embed-resources")]
pub mod embed {
    pub use rust_embed::*;
}

#[cfg(feature = "common")]
pub mod common;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
