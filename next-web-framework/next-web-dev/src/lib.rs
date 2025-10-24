pub mod application;
pub mod autoconfigure;
pub mod autoregister;
pub mod banner;
pub mod common;
pub mod configurer;
pub mod converter;
pub mod crypto;
pub mod error;
pub mod event;
pub mod extract;
pub mod interceptor;
pub mod macros;
pub mod manager;
pub mod middleware;
pub mod service;

pub mod stream;
pub mod util;

pub use crate::extract::required_header::header_names;


pub use axum::Router;
pub use axum::{body, error_handling, handler, http, response, routing};

pub use headers;
pub use rand;
pub use inventory::submit;

pub use rudi_dev::{Properties, SingleOwner, Singleton, Transient};

pub use next_web_core::*;
pub use next_web_macros::Idempotency;
pub use next_web_macros::{
    AnyMapping, DeleteMapping, GetMapping, PatchMapping, PostMapping, PutMapping, RequestMapping,
};

pub use next_web_macros::Desensitized;
pub use next_web_macros::{Builder, FieldName, GetSet, RequiredArgsConstructor};

#[cfg(feature = "enable-state-machine")]
pub mod state_machine;

#[cfg(feature = "enable-scheduling")]
pub use next_web_macros::Scheduled;

#[cfg(feature = "enable-web-security")]
pub use next_web_macros::PreAuthorize;
#[cfg(feature = "enable-web-security")]
pub use next_web_security as security;

#[cfg(feature = "enable-retry")]
pub use next_web_macros::Retryable;
#[cfg(feature = "enable-retry")]
pub use next_web_retry as retry;

#[cfg(feature = "enable-i18n")]
pub mod i18n;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
