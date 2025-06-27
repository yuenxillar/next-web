pub mod autoconfigure;
pub mod autoregister;
pub mod banner;
pub mod converter;
pub mod error;
pub mod event;
pub mod interceptor;
pub mod manager;
pub mod middleware;
pub mod util;
pub mod application;

mod tests;

pub use rudi_dev::{Singleton, Transient, SingleOwner, Properties};

#[cfg(feature = "job_scheduler")]
pub use tokio_cron_scheduler::Job;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;