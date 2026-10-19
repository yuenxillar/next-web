//! A client certificate, presented over TLS, through the whole security chain.
//!
//! The chain of this port never sees the TLS handshake: the container — or the
//! terminator in front of it — validates the certificate and hands the
//! authenticated identity to the application, which writes it into the request
//! attribute the X.509 filter reads. This test is that arrangement, whole: a
//! real TLS listener that asks for a client certificate, a real client that
//! presents one, and the chain of `HttpSecurity::build()` behind it.
//!
//! What is not proven here is the certificate *validation* itself: rustls does
//! that in the test, as a container would in production. The subject of the
//! certificate is likewise the container's to extract; this test writes the one
//! its client certificate carries, which it first checks the handshake agreed
//! on.

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse, RequestBody,
    },
};
use next_web_security::{
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        web::{builders::HttpSecurity, configurers::DefaultLoginPageConfigurer},
    },
    core::userdetails::User,
    provisioning::InMemoryUserDetailsManager,
    web::FilterChainProxy,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::{rustls, TlsAcceptor, TlsConnector};

/// The certificate of the test certificate authority.
const CA_CERT: &str = include_str!("fixtures/tls/ca-cert.pem");
/// The certificate of the TLS listener.
const SERVER_CERT: &str = include_str!("fixtures/tls/server-cert.pem");
/// The private key of the TLS listener.
const SERVER_KEY: &str = include_str!("fixtures/tls/server-key.pem");
/// The certificate the client presents.
const CLIENT_CERT: &str = include_str!("fixtures/tls/client-cert.pem");
/// The private key the client presents.
const CLIENT_KEY: &str = include_str!("fixtures/tls/client-key.pem");
/// The name of the user the client certificate is mapped to.
const USER: &str = "Alice";

/// Returns the certificates of a PEM document.
fn certificates(pem: &str) -> Vec<CertificateDer<'static>> {
    rustls_pemfile::certs(&mut pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .expect("the certificates of the fixture are valid")
}

/// Returns the private key of a PEM document.
fn private_key(pem: &str) -> PrivateKeyDer<'static> {
    rustls_pemfile::private_key(&mut pem.as_bytes())
        .expect("the private key of the fixture is valid")
        .expect("the fixture carries a private key")
}

/// Returns a TLS listener that accepts only clients with a certificate of the
/// test authority.
fn acceptor() -> TlsAcceptor {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates(CA_CERT) {
        roots
            .add(certificate)
            .expect("the authority certificate is a usable root");
    }
    let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .expect("the verifier is built");
    let config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certificates(SERVER_CERT), private_key(SERVER_KEY))
        .expect("the listener has its certificate and key");

    TlsAcceptor::from(Arc::new(config))
}

/// Returns a TLS connector that presents the client certificate.
fn connector() -> TlsConnector {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates(CA_CERT) {
        roots
            .add(certificate)
            .expect("the authority certificate is a usable root");
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(certificates(CLIENT_CERT), private_key(CLIENT_KEY))
        .expect("the client has its certificate and key");

    TlsConnector::from(Arc::new(config))
}

/// Returns a connector that presents no certificate.
fn anonymous_connector() -> TlsConnector {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates(CA_CERT) {
        roots
            .add(certificate)
            .expect("the authority certificate is a usable root");
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    TlsConnector::from(Arc::new(config))
}

/// The terminal of the chain: the application, which names the user it sees.
#[derive(Clone)]
struct Application;

#[async_trait]
impl HttpFilterChain for Application {
    async fn do_filter(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        use next_web_security::core::context::SecurityContextHolder;

        let name = SecurityContextHolder::get_context()
            .get_authentication()
            .map(|authentication| authentication.name().into_owned())
            .unwrap_or_else(|| "anonymous".to_string());
        response.insert_header("content-type", "text/html;charset=UTF-8");
        response.set_body(format!("<html><body><p id=\"user\">{name}</p></body></html>").into_bytes());

        Ok(())
    }
}

/// Returns a user details service that holds the user of the certificate.
fn user_details_service() -> InMemoryUserDetailsManager {
    InMemoryUserDetailsManager::with_users(vec![Arc::new(
        User::with_username(USER)
            .password(Some("unused".to_string()))
            .build(),
    )])
}

/// Returns an HTTP security that authenticates a client certificate.
fn http_security() -> HttpSecurity {
    let mut shared_objects: HashMap<std::any::TypeId, Box<dyn next_web_core::any::AnyClone>> =
        HashMap::new();
    shared_objects.insert(
        std::any::TypeId::of::<next_web_security::web::util::matcher::PathPatternRequestMatcherBuilder>(
        ),
        Box::new(
            next_web_security::web::util::matcher::PathPatternRequestMatcherBuilder::default(),
        ),
    );
    let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
    http.csrf(|_| {})
        .error_handling(|_| {})
        .headers(|_| {})
        .session_management(|_| {})
        .security_context(|_| {})
        .request_cache(|_| {})
        .anonymous(|_| {});
    http.with(DefaultLoginPageConfigurer::default(), |_| {});
    http.x509(|x509| {
        x509.user_details_service(user_details_service());
    });

    http
}

/// What the listener read from the client, and what it answered.
struct Exchange {
    /// The certificate the handshake agreed on, if the client presented one.
    peer_certificate: Option<CertificateDer<'static>>,
    /// The response the chain produced, as it was written to the client.
    response: String,
}

/// Serves one TLS connection with the given chain, and answers one request.
async fn serve_one(
    listener: &TcpListener,
    acceptor: TlsAcceptor,
    proxy: Arc<FilterChainProxy>,
    with_certificate: bool,
) -> std::io::Result<Exchange> {
    let (tcp, _) = listener.accept().await?;
    let mut stream = match acceptor.accept(tcp).await {
        Ok(stream) => stream,
        Err(error) => return Err(std::io::Error::other(error)),
    };

    // The request head, which is all this test sends.
    let mut head = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..read]);
        if head.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let peer_certificate = stream
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|certificates| certificates.first().cloned());

    let mut request = Request::builder()
        .method("GET")
        .uri("https://127.0.0.1/")
        .body(Body::empty())
        .expect("a valid request");
    request
        .extensions_mut()
        .insert(RequestBody::new(Vec::new()));
    // The container's part: it validated the certificate and hands it on, and the
    // filter that reads the attribute of a servlet-style request is what turns it
    // into the principal. The test inserts what the transport of an application
    // inserts — the certificate itself, not a subject it would have to trust.
    if with_certificate {
        if let Some(certificate) = peer_certificate.clone() {
            request
                .extensions_mut()
                .insert(next_web_core::http::PeerCertificate::new(
                    certificate.as_ref().to_vec(),
                ));
        }
    }

    let lifecycle = next_web_core::http::RequestLifecycle::begin(&mut request);
    let mut response = Response::default();
    proxy
        .do_filter(&mut request, &mut response, &Application)
        .await
        .expect("the chain runs");
    lifecycle.end(&mut request, &mut response);

    let status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the body is collected");
    let body = String::from_utf8_lossy(&body).to_string();
    let written = format!(
        "HTTP/1.1 {status} OK\r\ncontent-type: text/html;charset=UTF-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(written.as_bytes()).await?;
    stream.flush().await?;
    // A close_notify, so that a client reading to the end of the response sees
    // the end of the stream rather than a dropped connection.
    let _ = stream.shutdown().await;

    Ok(Exchange {
        peer_certificate,
        response: body,
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_certificate_authenticates_over_tls() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port is free");
    let address = listener.local_addr().expect("the address is known");
    let mut http = http_security();
    let chain = http.build();
    let proxy = Arc::new(FilterChainProxy::new(vec![Arc::new(chain)]));
    let acceptor = acceptor();
    let connector = connector();

    let server = {
        let proxy = proxy.clone();
        tokio::spawn(async move { serve_one(&listener, acceptor, proxy, true).await })
    };

    // The client: a TCP connection, a TLS handshake with a certificate, and one
    // request.
    let tcp = TcpStream::connect(address).await.expect("the listener answers");
    let server_name = ServerName::try_from("127.0.0.1").expect("an address is a server name");
    let mut stream = connector
        .connect(server_name, tcp)
        .await
        .expect("the handshake holds");
    stream
        .write_all(b"GET / HTTP/1.1\r\nhost: 127.0.0.1\r\nconnection: close\r\n\r\n")
        .await
        .expect("the request is written");
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .await
        .expect("the answer is read");
    let answer = String::from_utf8_lossy(&answer).to_string();

    let exchange = server.await.expect("the listener task runs").expect("the exchange holds");

    assert_eq!(
        exchange.peer_certificate.as_ref(),
        certificates(CLIENT_CERT).first(),
        "the handshake agreed on the certificate the client presented"
    );
    assert!(
        answer.starts_with("HTTP/1.1 200"),
        "the request is answered, the answer was: {answer}"
    );
    assert!(
        exchange.response.contains(&format!("<p id=\"user\">{USER}</p>")),
        "the application sees the user of the certificate, the page was: {}",
        exchange.response
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_without_a_certificate_is_refused_by_the_handshake() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port is free");
    let address = listener.local_addr().expect("the address is known");
    let mut http = http_security();
    let chain = http.build();
    let proxy = Arc::new(FilterChainProxy::new(vec![Arc::new(chain)]));
    let acceptor = acceptor();
    let connector = anonymous_connector();

    let server = tokio::spawn(async move { serve_one(&listener, acceptor, proxy, false).await });

    let tcp = TcpStream::connect(address).await.expect("the listener answers");
    let server_name = ServerName::try_from("127.0.0.1").expect("an address is a server name");
    let outcome = connector.connect(server_name, tcp).await;

    // TLS 1.3 lets the client believe the handshake is done before the server
    // rejects it, so the refusal can surface on the first read instead.
    let refused = match outcome {
        Err(_) => true,
        Ok(mut stream) => {
            let _ = stream
                .write_all(b"GET / HTTP/1.1\r\nhost: 127.0.0.1\r\nconnection: close\r\n\r\n")
                .await;
            let mut answer = Vec::new();
            stream.read_to_end(&mut answer).await.is_err() || answer.is_empty()
        }
    };

    assert!(
        refused,
        "a client without a certificate cannot finish the handshake"
    );
    // The listener refuses the connection before any request reaches the chain.
    let _ = server.await;
}
