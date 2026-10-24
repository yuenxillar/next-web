//! The background service example: a UDP echo server and a client that talks to
//! it while the web server runs.
//!
//! When the application starts, the manager picks up every service that
//! [`Application::background_services`] returns and runs each one on a task of its
//! own, so `run` never blocks the thread that starts the application. The manager
//! stops a service by cancelling its task and then calling the service's
//! [`BackgroundService::shutdown`], which is where a service releases whatever it
//! opened — the echo server, for instance, closes its socket there.
//!
//! Two services of the example talk to each other over UDP. The echo server answers
//! every datagram with the payload it received, and the client sends a datagram
//! every two seconds and logs the answer. A datagram from any other program is
//! answered as well, so the server is easy to try by hand.
//!
//! Try it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin background_service
//! # the log of the application shows the datagram and the answer of every two
//! # seconds; a datagram of another program is answered as well:
//! $udp = [System.Net.Sockets.UdpClient]::new()
//! $udp.Connect('127.0.0.1', 8080)
//! [void]$udp.Send([Text.Encoding]::ASCII.GetBytes('ping'), 4)
//! [Text.Encoding]::ASCII.GetString($udp.Receive([ref]$null))
//! ```

use std::{net::SocketAddr, sync::Arc, time::Duration};

use next_web::{
    Application, NextWebApplication,
    core::async_trait,
    core::service::{BackgroundService, HealthStatus, ServiceError},
};
use tokio::net::UdpSocket;

/// The address the echo server listens on, which the environment can override.
const DEFAULT_ADDRESS: &str = "127.0.0.1:8080";
/// The variable that names the address the echo server listens on.
const ADDRESS_VARIABLE: &str = "NEXT_UDP_ADDRESS";
/// The largest datagram the echo server reads.
const BUFFER_SIZE: usize = 64 * 1024;
/// The time between two datagrams of the client.
const INTERVAL: Duration = Duration::from_secs(2);

#[derive(Default)]
struct EchoApplication;

impl Application for EchoApplication {
    fn background_services(
        &self,
        _ctx: &mut dyn next_web::ConfigurableApplicationContext,
    ) -> Vec<Arc<dyn BackgroundService>> {
        let address = std::env::var(ADDRESS_VARIABLE)
            .unwrap_or_else(|_| String::from(DEFAULT_ADDRESS))
            .parse()
            .expect("the address of the echo server is an address");

        vec![
            Arc::new(EchoService::new(address)),
            Arc::new(EchoClient::new(address)),
        ]
    }
}

/// A UDP server that answers every datagram with the payload it received.
struct EchoService {
    /// The address the server listens on.
    address: SocketAddr,
}

impl EchoService {
    /// Creates the server, which listens on `address`.
    ///
    /// # Arguments
    ///
    /// * `address` - The address the socket of the server is bound to.
    fn new(address: SocketAddr) -> Self {
        Self { address }
    }
}

#[async_trait]
impl BackgroundService for EchoService {
    fn name(&self) -> &'static str {
        "echo"
    }

    async fn run(self: Arc<Self>) -> Result<(), ServiceError> {
        let socket = UdpSocket::bind(self.address).await?;
        println!("the echo server listens on {}\n", socket.local_addr()?);

        let mut buffer = vec![0u8; BUFFER_SIZE];
        loop {
            // The token of the manager is what ends the loop when the application
            // stops, so the socket is closed by the service and not by the drop of
            // a task the manager gave up on.
            let received = socket.recv_from(&mut buffer).await;

            let (length, peer) = match received {
                Ok(received) => received,
                Err(error) => {
                    // A datagram of a peer that closed its socket makes the read
                    // fail on Windows; the server keeps listening.
                    eprintln!("a datagram was not read: {error}");
                    continue;
                }
            };

            println!("a datagram was received from {peer}, length {length}");

            if let Err(error) = socket.send_to(&buffer[..length], peer).await {
                eprintln!("the answer of a datagram was not sent to {peer}: {error}");
            }
        }
    }

    async fn shutdown(&self) {
        println!("the echo server stopped");
    }

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

/// Sends a datagram to the echo server every [`INTERVAL`] and logs the answer.
struct EchoClient {
    /// The address of the echo server.
    address: SocketAddr,
}

impl EchoClient {
    /// Creates the client of the echo server at `address`.
    ///
    /// # Arguments
    ///
    /// * `address` - The address of the echo server.
    fn new(address: SocketAddr) -> Self {
        Self { address }
    }
}

#[async_trait]
impl BackgroundService for EchoClient {
    fn name(&self) -> &'static str {
        "echo client"
    }

    async fn run(self: Arc<Self>) -> Result<(), ServiceError> {
        let socket = UdpSocket::bind("127.0.0.1:0").await?;
        let mut interval = tokio::time::interval(INTERVAL);
        let mut answer = vec![0u8; BUFFER_SIZE];
        let mut sent = 0u32;

        loop {
            let _ = interval.tick().await;
            sent = sent.wrapping_add(1);
            let payload = format!("hello {sent}");

            if let Err(error) = socket.send_to(payload.as_bytes(), self.address).await {
                eprintln!("the datagram of the client was not sent: {error}");
                continue;
            }

            // The answer is read with a bound, so an echo server that is not
            // there does not keep the client from sending the next datagram.
            match tokio::time::timeout(Duration::from_secs(1), socket.recv(&mut answer)).await {
                Ok(Ok(length)) => println!(
                    "the echo server answered datagram {sent}: {}",
                    String::from_utf8_lossy(&answer[..length])
                ),
                Ok(Err(error)) => eprintln!("the answer was not read: {error}"),
                Err(_) => eprintln!("the echo server did not answer datagram {sent}"),
            }
        }
    }

    async fn shutdown(&self) {
        println!("the echo client stopped");
    }

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<EchoApplication>::default().run().await
}
