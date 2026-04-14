#![allow(missing_docs)]

//! RabbitMQ starter for Next Web.
//!
//! This crate follows the same "drop in and auto-configure" style as
//! `next-web-websocket`: define properties, implement a configurer, and the
//! framework will initialize the RabbitMQ connection plus listeners for you.

pub mod autoconfigure;
pub mod config;
pub mod core;
pub mod service;

pub use amqprs::{
    BasicProperties,
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicNackArguments, Channel, ConsumerMessage,
    },
    connection::Connection,
};
