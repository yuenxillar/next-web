use std::sync::LazyLock;

use tokio::sync::broadcast::Sender;

/// The application started signal
pub static APPLICATION_STARTED_SIGNAL: LazyLock<Sender<()>> = LazyLock::new(|| {
    let (sender, _receiver) = tokio::sync::broadcast::channel::<()>(30);

    sender
});

/// The graceful shutdown signal
pub(crate) static APPLICATION_GRACEFUL_SHUTDOWN_SIGNAL: LazyLock<Sender<()>> =
    LazyLock::new(|| {
        let (sender, _receiver) = tokio::sync::broadcast::channel::<()>(30);

        sender
    });
