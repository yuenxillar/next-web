//! Default [`ApplicationContextFactory`] implementation that creates an
//! appropriate application context.

use next_web_core::env::ConfigurableEnvironment;

use crate::application_context_factory::ApplicationContextFactory;
use crate::configurable_application_context::ConfigurableApplicationContext;
use crate::context::{ApplicationEnvironment, DefaultApplicationContext};

/// Default [`ApplicationContextFactory`] implementation.
///
/// It delegates to a list of candidate factories and falls back to a default
/// context creation strategy when none of them produces a result.
#[derive(Default)]
pub struct DefaultApplicationContextFactory;

impl DefaultApplicationContextFactory {}

impl ApplicationContextFactory for DefaultApplicationContextFactory {
    fn create(&self) -> Option<Box<dyn ConfigurableApplicationContext>> {
        Some(Box::new(DefaultApplicationContext::default()))
    }

    fn create_environment(&self) -> Option<Box<dyn ConfigurableEnvironment>> {
        Some(Box::new(ApplicationEnvironment::default()))
    }
}
