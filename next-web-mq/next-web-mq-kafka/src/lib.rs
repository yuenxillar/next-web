#![allow(missing_docs)]

//! Kafka starter for Next Web.
//!
//! This crate follows the same module layout as the existing `next-web`
//! starters:
//! - `autoconfigure`: properties and bootstrap logic
//! - `config`: listener registration contracts
//! - `core`: messaging abstractions
//! - `service`: producer-facing template/service facade

pub mod autoconfigure;
pub mod config;
pub mod core;
pub mod service;

pub use crate::{
    autoconfigure::{
        kafka_auto_configuration::KafkaAutoConfiguration, kafka_properties::KafkaProperties,
    },
    config::{
        kafka_configurer::KafkaConfigurer, kafka_listener_registration::KafkaListenerRegistration,
        kafka_listener_registry::KafkaListenerRegistry,
    },
    core::{
        endpoint::{KafkaEndpoint, KafkaListenerEndpoint},
        listener::kafka_listener::KafkaListener,
        record::{ConsumerRecord, ProducerRecord},
    },
    service::kafka_template::KafkaTemplate,
};
