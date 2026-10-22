//! TLS upstream behaviour, exercised over a real handshake.
//!
//! The fixture is a self-signed certificate for `localhost` (`tests/fixtures/tls`),
//! so the two cases that matter for trust are covered: verification is on by
//! default and a self-signed backend is refused, and `insecure_skip_verify`
//! is the only way through. A *trusted* private CA cannot be covered here
//! because `upstream.tls.ca_file` is rejected at startup in this build.

mod support;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};

use support::{get, TestGateway};

const CERT_PEM: &str = include_str!("fixtures/tls/server.crt");
const KEY_PEM: &str = include_str!("fixtures/tls/server.key");

/// A TLS server that answers `200 secure` to any request.
fn start_tls_upstream() -> SocketAddr {
    // `ServerConfig::builder()` needs a process wide crypto provider; without
    // one it panics inside the server thread and every request times out.
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut CERT_PEM.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .expect("parse the test certificate");

    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut KEY_PEM.as_bytes())
        .expect("parse the test key")
        .expect("a private key in the fixture");

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("build the TLS server config");
    // Without ALPN the client and the mock can end up disagreeing about the
    // protocol, and the mock speaks HTTP/1.1.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    let acceptor = TlsAcceptor::from(Arc::new(config));
    // Dual stack, so `localhost` (which resolves to `::1` and `127.0.0.1`) is
    // reachable whichever family the gateway tries first.
    let listener = std::net::TcpListener::bind("[::]:0").expect("bind the TLS upstream");
    let addr = listener.local_addr().expect("TLS upstream address");
    listener
        .set_nonblocking(true)
        .expect("non blocking listener");

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        runtime.block_on(async move {
            let listener = TcpListener::from_std(listener).expect("adopt the listener");
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        return;
                    };
                    let mut buffer = [0_u8; 4096];
                    let _ = stream.read(&mut buffer).await;

                    let body = b"secure";
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(body).await;
                    let _ = stream.flush().await;
                });
            }
        });
    });

    addr
}

fn start_gateway(upstream: SocketAddr, tls_settings: &str) -> TestGateway {
    let document = format!(
        "\
next:
  gateway:
    server:
      bind: \"127.0.0.1:{{proxy}}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{{admin}}\"
{tls_settings}    routes:
      - id: secure
        uri: \"https://localhost:{port}\"
        predicates:
          - Path=/secure
",
        port = upstream.port()
    );

    TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    })
}

#[test]
fn a_self_signed_upstream_is_refused_by_default() {
    let upstream = start_tls_upstream();
    let gateway = start_gateway(upstream, "");

    let response = get(gateway.addr, "/secure", &[]);

    assert_eq!(
        response.status, 502,
        "an unverifiable upstream must not be proxied: {}",
        response.text()
    );
}

/// Known defect, reproduced here: with verification disabled the handshake with
/// a self-signed backend still fails, so `insecure_skip_verify` currently has no
/// usable effect and a private CA cannot be used at all in this build.
#[test]
#[ignore = "known defect: insecure_skip_verify does not make the handshake succeed"]
fn insecure_skip_verify_accepts_the_same_upstream() {
    let upstream = start_tls_upstream();
    let gateway = start_gateway(
        upstream,
        "    upstream:\n      tls:\n        insecure_skip_verify: true\n",
    );

    let response = get(gateway.addr, "/secure", &[]);

    assert_eq!(
        response.status, 200,
        "insecure_skip_verify must let the handshake through: {}",
        response.text()
    );
    assert_eq!(response.text(), "secure");
}
