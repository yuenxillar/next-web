use std::{error::Error, future::Future, net::SocketAddr};

use axum::Router;

use crate::web::server::Server;

pub struct WebServer {
    socket_addr: SocketAddr,
    router: Option<Router>,
}

impl WebServer {
    pub fn new<A>(socket_addr: A, router: Router) -> Self
    where
        A: Into<SocketAddr>,
    {
        Self {
            socket_addr: socket_addr.into(),
            router: Some(router),
        }
    }
}

impl Server for WebServer {
    fn run<'a>(&'a mut self) -> impl Future<Output = Result<(), Box<dyn Error>>> + 'a {
        async move {
            let listener = tokio::net::TcpListener::bind(&self.socket_addr).await?;
            let app = self.router.take().unwrap_or_else(Router::new);
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(graceful_shutdown())
            .await?;

            Ok(())
        }
    }
}

/// Completes when the application receives a signal that asks it to shut down.
///
/// # Returns
///
/// The name of the signal that was received.
pub(crate) async fn shutdown_signal() -> &'static str {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        // A `SignalKind` is not a future: it has to be installed as a signal stream first, and
        // that installation can fail. Falling back to `pending` keeps the select arm quiet instead
        // of turning a broken handler into an immediate shutdown at startup.
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!(
                    %error,
                    "failed to install the SIGTERM handler; shutdown falls back to Ctrl+C"
                );
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => "Ctrl_C SIGINT",
        _ = terminate => "Terminate SIGTERM",
    }
}

/// Waits for a shutdown signal and reports it.
pub(crate) async fn graceful_shutdown() {
    shutdown_application().await;
}

/// Waits for either an operating-system signal or an explicit application
/// shutdown request, then runs the registered application cleanup callbacks.
pub(crate) async fn shutdown_application() {
    let Some(shutdown_hook) = crate::next_web_application::APPLICATION_SHUTDOWN_HOOK
        .get()
        .cloned()
    else {
        let reason = shutdown_signal().await;
        tracing::info!("Graceful shutdown of application completed, reason: {reason}");
        return;
    };

    if shutdown_hook.is_shutdown() {
        shutdown_hook.shutdown().await;
        shutdown_hook.wait_for_completion().await;
        return;
    }

    tokio::select! {
        reason = shutdown_signal() => {
            tracing::info!("Application shutdown requested, reason: {reason}");
            shutdown_hook.shutdown().await;
            shutdown_hook.wait_for_completion().await;
        }
        _ = shutdown_hook.cancelled() => {
            tracing::info!("Application shutdown requested through its shutdown hook.");
            shutdown_hook.shutdown().await;
            shutdown_hook.wait_for_completion().await;
        }
    }
}
