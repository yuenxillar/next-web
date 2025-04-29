pub mod core;
pub mod auto_register;
pub mod properties;
pub mod service;

pub use amqprs::{BasicProperties, channel::{ConsumerMessage, BasicConsumeArguments}};