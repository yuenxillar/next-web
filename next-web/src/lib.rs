pub mod application;
pub mod autoconfigure;
pub mod autoregister;
pub mod configurer;
pub mod context;
pub mod crypto;
pub mod diagnostics;
pub mod env;
pub mod error;
pub mod event;
pub mod extract;
pub mod interceptor;
pub mod macros;
pub mod manager;
pub mod signal;
pub mod stream;
pub mod support;
pub mod util;
pub mod web;

mod ansi;
mod application_arguments;
mod application_banner_printer;
mod application_context_factory;
mod application_context_initializer;
mod application_event_handler;
mod application_info_property_source;
mod application_properties;
mod application_resources;
mod application_runner;
mod application_shutdown_hook;
mod banner;
mod configurable_application_context;
mod default_application_arguments;
mod default_application_context_factory;
mod error_handler;
mod next_web_application;
mod next_web_banner;
mod next_web_error_reporter;
mod next_web_version;
mod resource_banner;
mod startup_info_logger;

pub use ansi::{AnsiBackground, AnsiColor, AnsiElement, AnsiPropertySource, AnsiStyle};
pub use application_arguments::ApplicationArguments;
pub(crate) use application_banner_printer::ApplicationBannerPrinter;
pub use application_context_factory::ApplicationContextFactory;
pub use application_context_initializer::ApplicationContextInitializer;
pub(crate) use application_event_handler::{ApplicationEventHandler, Event};
pub use application_info_property_source::ApplicationInfoPropertySource;
pub(crate) use application_properties::ApplicationProperties;
pub use application_resources::ApplicationResources;
pub use application_runner::ApplicationRunner;
pub use application_shutdown_hook::ApplicationShutdownHook;
pub use banner::Banner;
pub use configurable_application_context::ConfigurableApplicationContext;
pub use default_application_arguments::DefaultApplicationArguments;
pub(crate) use default_application_context_factory::DefaultApplicationContextFactory;
pub use error_handler::ErrorHandler;
pub use next_web_application::{Application, NextWebApplication};
pub(crate) use next_web_banner::NextWebBanner;
pub use next_web_error_reporter::NextWebErrorReporter;
pub use next_web_version::NextWebVersion;
pub use resource_banner::ResourceBanner;
pub use startup_info_logger::StartupInfoLogger;

pub use crate::extract::required_header::header_names;

pub use axum::Router;
pub use axum::{body, error_handling, handler, http, response, routing};

pub use rand;
pub use validator as validate;

pub use next_web_core as core;

#[cfg(feature = "enable-state-machine")]
pub use next_web_state_machine as state_machine;

#[cfg(feature = "enable-web-security")]
pub use next_web_security as security;

#[cfg(feature = "enable-websocket")]
pub use next_web_websocket as ws;

#[cfg(feature = "enable-retry")]
pub use next_web_retry as retry;

#[cfg(feature = "enable-api-doc")]
pub use next_web_api_doc as api_doc;

#[cfg(feature = "enable-i18n")]
pub mod i18n;

#[cfg(feature = "embed-resources")]
pub mod embed {
    pub use rust_embed::*;
}

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
