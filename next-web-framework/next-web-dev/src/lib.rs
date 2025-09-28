pub mod application;
pub mod autoconfigure;
pub mod autoregister;
pub mod banner;
pub mod common;
pub mod converter;
pub mod error;
pub mod event;
pub mod extract;
pub mod interceptor;
pub mod macros;
pub mod manager;
pub mod middleware;
pub mod service;
pub mod state_machine;
pub mod stream;
pub mod util;

pub use axum::extract::{Form, Json, Path, Query, State};
pub use axum::{body, error_handling, handler, http, response, routing};
pub use axum::{extract as axum_extract, middleware as axum_middleware};
pub use axum::{Extension, Router};
pub use inventory::submit;

pub use next_web_macros::{
    AnyMapping, DeleteMapping, GetMapping, PatchMapping, PostMapping, PutMapping, RequestMapping,
};
pub use next_web_macros::{Builder, Desensitized, FieldName, GetSet, RequiredArgsConstructor};
pub use next_web_macros::{Retryable, Scheduled};

pub use next_web_core::*;

#[cfg(feature = "i18n")]
pub mod i18n;

#[cfg(feature = "scheduler")]
pub use tokio_cron_scheduler::Job;

pub use rudi_dev::{Properties, SingleOwner, Singleton, Transient};

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
