use std::{sync::Arc, time::Duration};

use next_web_core::{env::ConfigurableEnvironment, io::ResourceLoader};

use crate::ConfigurableApplicationContext;

pub trait ApplicationEventHandler {
    fn handle_event(&mut self, event: Event);
}

pub enum Event<'a> {
    Starting {
        args: &'a [String],
    },
    EnvironmentPrepared(EnvironmentPreparedPayload<'a>),
    ContextPrepared {
        args: &'a [String],
        context: &'a mut dyn ConfigurableApplicationContext,
    },
    ContextLoaded {
        args: &'a [String],
        context: &'a mut dyn ConfigurableApplicationContext,
    },
    Started {
        args: &'a [String],
        context: &'a mut dyn ConfigurableApplicationContext,
        time_taken: Option<Duration>,
    },
    Ready {
        args: &'a [String],
        context: &'a mut dyn ConfigurableApplicationContext,
        time_taken: Option<Duration>,
    },
    Error {
        args: &'a [String],
        err: &'a (dyn std::error::Error + 'static),
    },
}

pub struct EnvironmentPreparedPayload<'a> {
    pub args: &'a [String],
    pub environment: &'a mut dyn ConfigurableEnvironment,
    pub resource_loader: Option<Arc<dyn ResourceLoader>>,
    pub additional_profiles: Vec<String>,
}
