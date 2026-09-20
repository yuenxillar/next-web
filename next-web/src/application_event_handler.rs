use std::time::Duration;

use next_web_core::env::ConfigurableEnvironment;

use crate::ConfigurableApplicationContext;

pub trait ApplicationEventHandler {
    fn handle_event(&mut self, event: Event);
}

pub enum Event<'a> {
    Starting {
        args: &'a [String],
    },
    EnvironmentPrepared {
        args: &'a [String],
        environment: &'a mut dyn ConfigurableEnvironment,
    },
    ContextPrepared {
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
        context: &'a mut dyn ConfigurableApplicationContext,
        err: &'a (dyn std::error::Error + 'static),
    },
}
