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
        tokio::signal::unix::SignalKind::terminate()
            .then(|signal| signal.recv())
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => "Ctrl_C SIGINT",
        _ = terminate => "Terminate SIGTERM",
    }
}

/// Waits for a shutdown signal and reports it.
async fn graceful_shutdown() {
    let reason = shutdown_signal().await;

    tracing::info!("Graceful shutdown of application completed, reason: {reason}")
}
