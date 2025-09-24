pub mod service;
pub mod macros;
pub mod stream;
pub mod autoconfigure;
pub mod autoregister;
pub mod banner;
pub mod converter;
pub mod error;
pub mod extract;
pub mod event;
pub mod interceptor;
pub mod manager;
pub mod middleware;
pub mod util;
pub mod application;
pub mod state_machine;
pub mod common;

pub use axum::*;
pub use inventory::submit;

pub use next_web_macros::{GetSet, Builder, FieldName, Desensitized};
pub use next_web_macros::{RequestMapping, GetMapping, PostMapping, PutMapping, DeleteMapping, PatchMapping, AnyMapping};
pub use next_web_macros::{Retryable, Scheduled};

pub use next_web_core::*;

#[cfg(feature = "i18n")]
pub mod i18n;

#[cfg(feature = "scheduler")]
pub use tokio_cron_scheduler::Job;

pub use rudi_dev::{Singleton, Transient, SingleOwner, Properties};

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;