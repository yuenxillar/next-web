use std::time::Duration;

use notify::{EventHandler, RecommendedWatcher, Watcher as MyWatcher};

pub struct Watcher(RecommendedWatcher);

impl Watcher {
    pub fn new<F: EventHandler>(event_handler: F) -> Result<Self, notify::Error> {
        RecommendedWatcher::new(event_handler, Default::default())
            .map(|watcher| Watcher(watcher))
            .map_err(|e| e.into())
    }

    pub fn watch<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        recursive: bool,
    ) -> Result<(), notify::Error> {
        self.0.watch(
            path.as_ref(),
            if recursive {
                notify::RecursiveMode::Recursive
            } else {
                notify::RecursiveMode::NonRecursive
            },
        )
    }

    pub fn unwatch<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        self.0.unwatch(path.as_ref())
    }

    pub fn configure(
        &mut self,
        poll_interval: Option<Duration>,
        compare_contents: bool,
        follow_symlinks: bool,
    ) -> Result<bool, notify::Error> {
        let config = notify::Config::default()
        .with_compare_contents(compare_contents)
        .with_follow_symlinks(follow_symlinks);
        poll_interval.map(|c| config.with_poll_interval(c));
        self.0.configure(config)
    }
}
