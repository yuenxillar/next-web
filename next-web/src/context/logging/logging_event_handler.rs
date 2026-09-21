use crate::{ApplicationEventHandler, Event};

#[derive(Default)]
pub struct LoggingEventHandler;

impl LoggingEventHandler {
    fn on_starting(&mut self, _args: &[String]) {
        let default_format = "%Y-%m-%d %H:%M:%S%.3f";
        let config = tracing_subscriber::fmt::format()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                default_format.to_string(),
            ))
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_ansi(true)
            .with_source_location(true)
            .with_thread_names(true);

        // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

        let logger = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_ansi(false)
            .event_format(config);

        logger.init();
    }
}

impl ApplicationEventHandler for LoggingEventHandler {
    fn handle_event(&mut self, event: crate::Event) {
        match event {
            Event::Starting { args } => self.on_starting(args),
            _ => {}
        }
    }
}
