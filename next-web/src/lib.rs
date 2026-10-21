pub mod autoconfigure;
pub mod autoregister;
pub mod context;
pub mod diagnostics;
pub mod env;
pub mod extract;
pub mod macros;
pub mod manager;
pub mod resources;
pub mod scheduling;
pub mod security;
pub mod support;
pub mod util;
pub mod web;
pub mod wrapper;

mod ansi;
mod application_arguments;
mod application_banner_printer;
mod application_context_factory;
mod application_context_initializer;
mod application_environment;
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
mod environment_post_processor;
mod error_handler;
mod next_web_application;
mod next_web_banner;
mod next_web_error_reporter;
mod next_web_version;
mod resource_banner;
mod startup_info_logger;

pub use ansi::{AnsiBackground, AnsiColor, AnsiElement, AnsiPropertySource, AnsiStyle};
pub use application_arguments::ApplicationArguments;
pub use application_context_factory::ApplicationContextFactory;
pub use application_context_initializer::ApplicationContextInitializer;
pub use application_info_property_source::ApplicationInfoPropertySource;
pub use application_resources::ApplicationResources;
pub use application_runner::ApplicationRunner;
pub use application_shutdown_hook::ApplicationShutdownHook;
pub use banner::Banner;
pub use configurable_application_context::ConfigurableApplicationContext;
pub use default_application_arguments::DefaultApplicationArguments;
pub use environment_post_processor::EnvironmentPostProcessor;
pub use error_handler::ErrorHandler;
pub use next_web_application::{
    Application, ApplicationResult, ApplicationState, NextWebApplication,
};
pub use next_web_error_reporter::NextWebErrorReporter;
pub use next_web_version::NextWebVersion;
pub use resource_banner::ResourceBanner;
pub use startup_info_logger::StartupInfoLogger;

pub(crate) use application_banner_printer::ApplicationBannerPrinter;
pub(crate) use application_environment::ApplicationEnvironment;
pub(crate) use application_event_handler::*;
pub(crate) use application_properties::ApplicationProperties;
pub(crate) use default_application_context_factory::DefaultApplicationContextFactory;
pub(crate) use next_web_banner::NextWebBanner;

pub use crate::extract::required_header::header_names;

pub use axum::Router;
pub use axum::{body, error_handling, handler, http, response, routing};

pub use rand;
pub use validator;

pub use next_web_core as core;
pub use next_web_singletons as singletons;

#[cfg(feature = "enable-i18n")]
pub use web::i18n;

#[cfg(feature = "enable-state-machine")]
pub use next_web_state_machine as state_machine;

/// Marks a function as the handler of a state machine transition.
///
/// Pass `crate = next_web::state_machine` so the generated code refers to the
/// re-export of this crate instead of the state machine crate itself.
#[cfg(feature = "enable-state-machine")]
pub use next_web_macros::on_transition;

#[cfg(feature = "enable-websocket")]
pub use next_web_websocket as ws;

#[cfg(feature = "enable-retry")]
pub use next_web_retry as retry;

#[cfg(feature = "enable-resilience")]
pub use next_web_resilience as resilience;

#[cfg(feature = "enable-open-api")]
pub mod open_api;

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
