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


#[cfg(feature = "i18n")]
pub mod i18n;

mod tests;

pub use rudi_dev::{Singleton, Transient, SingleOwner, Properties};

#[cfg(feature = "scheduler")]
pub use tokio_cron_scheduler::Job;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;