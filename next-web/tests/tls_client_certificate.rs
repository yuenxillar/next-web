//! A TLS connection that presents a client certificate hands it to the
//! application.
//!
//! The security layer cannot read a certificate out of an HTTP request: the TLS
//! server of the framework terminates the connection, and it is that server's
//! job to hand the certificate on. This test serves a router over TLS that asks
//! for a client certificate, and checks that the request of a client which
//! presents one carries it — and that a client which presents none is refused by
//! the handshake.

#![cfg(feature = "tls-rustls")]

use std::{
    net::{SocketAddr, TcpListener as StdTcpListener},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{Router, extract::Request, routing::get};
use next_web::web::server::TlsWebServer;
use next_web_core::http::PeerCertificate;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};

/// Returns the path of a fixture of the TLS connections.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../next-web-security/tests/fixtures/tls")
        .join(name)
}

/// Returns the certificates of a PEM document.
fn certificates(name: &str) -> Vec<CertificateDer<'static>> {
    let pem = std::fs::read(fixture(name)).expect("the fixture is read");

    rustls_pemfile::certs(&mut pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .expect("the fixture is a PEM document")
}

/// Returns the private key of a PEM document.
fn private_key(name: &str) -> PrivateKeyDer<'static> {
    let pem = std::fs::read(fixture(name)).expect("the fixture is read");

    rustls_pemfile::private_key(&mut pem.as_slice())
        .expect("the fixture is a PEM document")
        .expect("the fixture holds a private key")
}

/// Returns a configuration of a server that requires a client certificate.
fn server_config() -> Arc<rustls::ServerConfig> {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates("ca-cert.pem") {
        roots.add(certificate).expect("the authority is a root");
    }
    let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .expect("the verifier is built");
    let config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certificates("server-cert.pem"), private_key("server-key.pem"))
        .expect("the listener has its certificate and key");

    Arc::new(config)
}

/// Returns a connector that presents the client certificate when asked to.
fn connector(with_certificate: bool) -> tokio_rustls::TlsConnector {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates("ca-cert.pem") {
        roots.add(certificate).expect("the authority is a root");
    }
    let builder = rustls::ClientConfig::builder().with_root_certificates(roots);
    let config = if with_certificate {
        builder
            .with_client_auth_cert(
                certificates("client-cert.pem"),
                private_key("client-key.pem"),
            )
            .expect("the client has its certificate and key")
    } else {
        builder.with_no_client_auth()
    };

    tokio_rustls::TlsConnector::from(Arc::new(config))
}

/// Returns an address a server can bind to, and that this test therefore knows.
fn free_address() -> SocketAddr {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("a port is free");
    let address = listener.local_addr().expect("the address is known");
    drop(listener);

    address
}

/// Returns a router that answers with the certificate the request carries.
fn router() -> Router {
    Router::new().route(
        "/",
        get(|request: Request| async move {
            match request.extensions().get::<PeerCertificate>() {
                Some(certificate) => format!("certificate: {} bytes", certificate.encoded().len()),
                None => "certificate: none".to_string(),
            }
        }),
    )
}

/// Sends one request over TLS, and returns the body of the answer, or the error
/// that ended the connection.
async fn ask(address: SocketAddr, with_certificate: bool) -> Result<String, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let stream = tokio::net::TcpStream::connect(address)
        .await
        .map_err(|error| error.to_string())?;
    let server_name = ServerName::try_from("127.0.0.1").map_err(|error| error.to_string())?;
    let mut stream = connector(with_certificate)
        .connect(server_name, stream)
        .await
        .map_err(|error| error.to_string())?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nhost: 127.0.0.1\r\nconnection: close\r\n\r\n")
        .await
        .map_err(|error| error.to_string())?;
    let mut answer = Vec::new();
    let outcome = stream.read_to_end(&mut answer).await;
    if outcome.is_err() && answer.is_empty() {
        return Err("the connection was closed".to_string());
    }

    Ok(String::from_utf8_lossy(&answer).to_string())
}

/// Waits until the server of the address answers, or gives up.
async fn wait_until_serving(address: SocketAddr) {
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(address).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("the server of {address} never started");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_certificate_of_the_connection_reaches_the_router() {
    let address = free_address();
    let mut server = TlsWebServer::from_config(address, router(), server_config());
    let serving = tokio::spawn(async move {
        use next_web::web::server::Server;

        let _ = server.run().await;
    });
    wait_until_serving(address).await;

    let answer = ask(address, true).await.expect("the client is answered");

    assert!(
        answer.starts_with("HTTP/1.1 200"),
        "the request is answered, the answer was: {answer}"
    );
    assert!(
        answer.contains("certificate: ") && !answer.contains("certificate: none"),
        "the request carries the certificate of the client, the answer was: {answer}"
    );

    serving.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_without_a_certificate_is_refused() {
    let address = free_address();
    let mut server = TlsWebServer::from_config(address, router(), server_config());
    let serving = tokio::spawn(async move {
        use next_web::web::server::Server;

        let _ = server.run().await;
    });
    wait_until_serving(address).await;

    let answer = ask(address, false).await;

    assert!(
        answer.is_err(),
        "a client without a certificate is refused, the answer was: {answer:?}"
    );

    serving.abort();
}
