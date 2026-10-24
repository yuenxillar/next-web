//! RocketMQ starter for Next Web.
//!
//! This crate follows a Spring-style starter layout so applications can depend
//! on a small, well-documented entry point and let auto-configuration assemble
//! the rest:
//! - [`autoconfigure`] loads properties and creates the default starter
//!   services.
//! - [`config`] collects listener declarations from application configurers.
//! - [`core`] defines transport-neutral RocketMQ endpoint, message, and
//!   listener contracts.
//! - [`service`] exposes [`RocketmqTemplate`], the producer-facing facade that
//!   application code can inject directly.
//!
//! The current implementation focuses on starter structure, configuration
//! contracts, and framework-facing abstractions. It keeps the public API close
//! to the existing Kafka and RabbitMQ starters so the three modules behave
//! consistently.

pub mod autoconfigure;
pub mod config;
pub mod core;
pub mod service;

pub use crate::{
    autoconfigure::{
        rocketmq_auto_configuration::RocketmqAutoConfiguration,
        rocketmq_properties::RocketmqProperties,
    },
    config::{
        rocketmq_configurer::RocketmqConfigurer,
        rocketmq_listener_registration::RocketmqListenerRegistration,
        rocketmq_listener_registry::RocketmqListenerRegistry,
    },
    core::{
        endpoint::{RocketmqEndpoint, RocketmqListenerEndpoint, RocketmqMessageModel},
        listener::rocketmq_listener::RocketmqListener,
        message::{RocketmqDelivery, RocketmqMessage},
    },
    service::rocketmq_template::RocketmqTemplate,
};
