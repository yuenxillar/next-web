pub mod application;
pub mod autoconfigure;
pub mod autoregister;
pub mod banner;
pub mod converter;
pub mod error;
pub mod event;
pub mod interceptor;
pub mod manager;
pub mod middleware;
pub mod security;
pub mod transaction;
pub mod util;
pub mod router;

mod tests;

extern crate bcrypt;
extern crate anyhow;

pub extern crate chrono;
pub extern crate tracing;
pub extern crate once_cell;

pub use rudi::Singleton as Component;
pub use rudi::Transient as Prototype;
pub use rudi::Context   as ApplicationContext;


#[cfg(feature = "redis_enabled")]
pub extern crate deadpool_redis;
#[cfg(feature = "job_scheduler")]
pub extern crate tokio_cron_scheduler;