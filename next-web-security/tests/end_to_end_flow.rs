//! The whole flow, over a real socket.
//!
//! Each test builds the security configuration the way an application does —
//! the default configurers an application gets, plus the ones it chooses — serves
//! the chain the way the web layer does (a real `HttpSecurity::build()` chain
//! behind a real `FilterChainProxy`, on a real `TcpListener`), and drives it with
//! HTTP requests and a cookie jar.
//!
//! The virtual authenticator below plays the browser's part in a ceremony: it
//! reads the options the server issued, builds the client data and the
//! authenticator data the specification defines, and signs with a key it holds.
//! What it does not prove is the shipped script — no browser runs here — but it
//! does prove every request and every check between the browser and the user.

use std::{collections::HashMap, sync::Arc};

use axum::{body::Body, http::Request, response::Response, Router};
use base64::Engine;
use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
};
use next_web_security::{
    authentication::dao::DaoAuthenticationProvider,
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        web::{
            builders::HttpSecurity, configurers::DefaultLoginPageConfigurer, HttpSecurityBuilder,
        },
    },
    core::userdetails::{User, UserDetailsService},
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
    provisioning::InMemoryUserDetailsManager,
    web::FilterChainProxy,
};
use p256::ecdsa::SigningKey;
use tokio::net::TcpListener;

/// The password of the user every test signs in as.
const PASSWORD: &str = "password";

/// The name of the user every test signs in as.
const USER: &str = "Max";

/// The identifier of the credential the virtual authenticator registers.
const CREDENTIAL_ID: &[u8] = b"the-credential";

/// The terminal of the chain: the application itself.
///
/// The application answers inside the chain, the way a route of the application
/// does; the security context is still held then, and the response it writes is
/// the one the chain returns.
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
        let page = format!("<html><body><p id=\"user\">{name}</p></body></html>");
        response.insert_header("content-type", "text/html;charset=UTF-8");
        response.set_body(page.into_bytes());

        Ok(())
    }
}

/// Returns the application a chain serves.
fn application(proxy: Arc<FilterChainProxy>) -> axum::routing::MethodRouter {
    axum::routing::any(move |request: Request<Body>| {
        let proxy = proxy.clone();
        async move {
            // The web layer buffers the body, because filters read it: a form
            // post carries its parameters there, and the csrf filter is one of
            // the filters that need them.
            let (parts, body) = request.into_parts();
            let bytes = axum::body::to_bytes(body, 1024 * 1024)
                .await
                .expect("the body is read");
            let mut request = Request::from_parts(parts, Body::from(bytes.clone()));
            request
                .extensions_mut()
                .insert(next_web_core::http::RequestBody::new(bytes.to_vec()));
            // The lifecycle is the web layer's: it prepares the per-request
            // state, and it tells the client about the session the request left
            // behind, which is what makes a login outlive its request.
            let lifecycle = next_web_core::http::RequestLifecycle::begin(&mut request);
            let mut response = Response::default();
            proxy
                .do_filter(&mut request, &mut response, &Application)
                .await
                .expect("the chain runs");
            lifecycle.end(&mut request, &mut response);

            response
        }
    })
}

/// Returns the shared objects an application hands its builder.
fn shared_objects() -> HashMap<std::any::TypeId, Box<dyn next_web_core::any::AnyClone>> {
    let mut shared_objects: HashMap<std::any::TypeId, Box<dyn next_web_core::any::AnyClone>> =
        HashMap::new();
    shared_objects.insert(
        std::any::TypeId::of::<next_web_security::web::util::matcher::PathPatternRequestMatcherBuilder>(
        ),
        Box::new(
            next_web_security::web::util::matcher::PathPatternRequestMatcherBuilder::default(),
        ),
    );

    shared_objects
}

/// Returns a user details service that holds the test user.
fn user_details_service() -> Arc<dyn UserDetailsService> {
    let hash = BCryptPasswordEncoder::new(4)
        .encode(Some(PASSWORD))
        .expect("the password is encoded")
        .expect("the encoder returns a hash");

    Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
        User::with_username(USER).password(Some(hash)).build(),
    )]))
}

/// Returns an HTTP security the way an application of this port configures one.
///
/// The configurers are the ones `HttpSecurityConfiguration` applies by default,
/// followed by the form login and the passkey login of the application.
fn http_security(origin: &str) -> HttpSecurity {
    let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects());
    http.csrf(|_| {})
        .error_handling(|_| {})
        .headers(|_| {})
        .session_management(|_| {})
        .security_context(|_| {})
        .request_cache(|_| {})
        .anonymous(|_| {});
    http.with(DefaultLoginPageConfigurer::default(), |_| {});
    http.logout(|_| {});
    http.form_login(|_| {});

    let user_details_service = user_details_service();
    http.web_authn(|web_authn| {
        web_authn.rp_id("127.0.0.1");
        web_authn.rp_name("The application");
        web_authn.allowed_origins([origin]);
        web_authn.user_details_service(user_details_service.clone());
    });

    let mut provider = DaoAuthenticationProvider::new(user_details_service);
    provider.set_password_encoder(Arc::new(BCryptPasswordEncoder::new(4)));
    http.authentication_provider(Arc::new(provider));

    http
}

/// Serves a chain, and returns the base URL it is served on.
///
/// The configuration is built from the URL, because a relying party has to know
/// the origin the ceremony runs on before the chain is built.
async fn serve(build: impl FnOnce(&str) -> HttpSecurity) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port is free");
    let address = listener.local_addr().expect("the address is known");
    let base = format!("http://{address}");
    let mut http = build(&base);
    let chain = http.build();
    let proxy = Arc::new(FilterChainProxy::new(vec![Arc::new(chain)]));
    let app = Router::new().fallback_service(application(proxy));

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    base
}

/// Returns a client that keeps the cookie of the session it was given.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("the client is built")
}

/// Returns the session cookie of a response, when the response set one.
fn session_cookie(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("NEXT_SESSION"))
        .map(|value| value.split(';').next().unwrap_or(value).to_string())
}

/// Returns the CSRF token the login page renders as a hidden input.
fn csrf_parameter(html: &str) -> String {
    let start = html
        .find("name=\"_csrf\"")
        .expect("the login page carries a csrf token");
    let rest = &html[start..];
    let value_start = rest.find("value=\"").expect("the input carries a value") + 7;
    let value_end = rest[value_start..]
        .find('"')
        .expect("the value is quoted");

    rest[value_start..value_start + value_end].to_string()
}

/// Returns the CSRF header the pages send with a JSON submission.
///
/// Both pages render the header as a JSON object; the generated login page pads
/// the colon, so the pieces between the quotes are read rather than the exact
/// spacing of the object.
fn csrf_header(page: &str) -> (String, String) {
    let start = page
        .find("{\"")
        .expect("the page renders the csrf header");
    let quotes: Vec<&str> = page[start..]
        .split('"')
        .skip(1)
        .step_by(2)
        .take(2)
        .collect();

    assert_eq!(
        quotes.len(),
        2,
        "the header object has a name and a value, page was: {page}"
    );

    (quotes[0].to_string(), quotes[1].to_string())
}

/// Encodes bytes as base64url without padding, the way the browser does.
fn base64_url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a base64url string without padding.
fn decode_base64_url(value: &str) -> Vec<u8> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .expect("the value is base64url")
}

/// The signing key of the virtual authenticator.
///
/// A fixed seed rather than a random key: a failure is the same failure twice.
fn authenticator_key() -> SigningKey {
    SigningKey::from_slice(&[0x11; 32]).expect("the seed is a valid P-256 scalar")
}

/// Returns the COSE key of the virtual authenticator, as a registration carries it.
fn cose_key(key: &SigningKey) -> Vec<u8> {
    let point = key.verifying_key().to_sec1_point(false);
    let bytes = point.as_bytes();

    cbor(&ciborium::value::Value::Map(vec![
        (integer(1), integer(2)),
        (integer(3), integer(-7)),
        (integer(-1), integer(1)),
        (integer(-2), ciborium::value::Value::Bytes(bytes[1..33].to_vec())),
        (
            integer(-3),
            ciborium::value::Value::Bytes(bytes[33..65].to_vec()),
        ),
    ]))
}

/// Returns a CBOR integer value.
fn integer(value: i64) -> ciborium::value::Value {
    ciborium::value::Value::Integer(value.into())
}

/// Encodes a CBOR value.
fn cbor(value: &ciborium::value::Value) -> Vec<u8> {
    let mut encoded = Vec::new();
    ciborium::ser::into_writer(value, &mut encoded).expect("the value serializes");

    encoded
}

/// Returns the attestation object of a registration of the virtual authenticator.
fn attestation_object(rp_id: &str, credential_id: &[u8], key: &SigningKey) -> Vec<u8> {
    let mut authenticator_data = sha256(rp_id.as_bytes());
    authenticator_data.push(0x41); // the user was present, and a credential is attested
    authenticator_data.extend_from_slice(&0u32.to_be_bytes());
    authenticator_data.extend_from_slice(&[0u8; 16]); // the aaguid
    authenticator_data.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
    authenticator_data.extend_from_slice(credential_id);
    authenticator_data.extend_from_slice(&cose_key(key));

    cbor(&ciborium::value::Value::Map(vec![
        (
            ciborium::value::Value::Text("fmt".to_string()),
            ciborium::value::Value::Text("none".to_string()),
        ),
        (
            ciborium::value::Value::Text("attStmt".to_string()),
            ciborium::value::Value::Map(vec![]),
        ),
        (
            ciborium::value::Value::Text("authData".to_string()),
            ciborium::value::Value::Bytes(authenticator_data),
        ),
    ]))
}

/// Returns the authenticator data of an authentication.
fn assertion_data(rp_id: &str, sign_count: u32) -> Vec<u8> {
    let mut authenticator_data = sha256(rp_id.as_bytes());
    authenticator_data.push(0x01); // the user was present
    authenticator_data.extend_from_slice(&sign_count.to_be_bytes());

    authenticator_data
}

/// Returns the client data of a ceremony.
fn client_data_json(kind: &str, challenge: &str, origin: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": kind,
        "challenge": challenge,
        "origin": origin,
        "crossOrigin": false,
    }))
    .expect("the client data serializes")
}

/// Returns the SHA-256 of a message.
fn sha256(message: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    Sha256::digest(message).to_vec()
}

/// Signs a message the way the authenticator signs a ceremony.
fn sign(key: &SigningKey, message: &[u8]) -> Vec<u8> {
    use p256::ecdsa::signature::Signer;

    let signature: p256::ecdsa::Signature = key.sign(message);

    signature.to_bytes().to_vec()
}

/// Signs the user in through the form, and returns the session cookie.
///
/// The page and the submission share one session: the token the page renders is
/// stored in it, so the submission carries the cookie the page was served with.
async fn sign_in_with_password(client: &reqwest::Client, base: &str) -> String {
    let page = client
        .get(format!("{base}/login"))
        .send()
        .await
        .expect("the login page is served");
    let status = page.status();
    let page_cookie = session_cookie(&page);
    let page = page.text().await.expect("the page is text");
    let page_cookie = page_cookie.unwrap_or_else(|| {
        panic!("the page starts a session, status {status}, page: {page}")
    });
    let csrf = csrf_parameter(&page);

    let response = client
        .post(format!("{base}/login"))
        .header("cookie", &page_cookie)
        .form(&[("username", USER), ("password", PASSWORD), ("_csrf", &csrf)])
        .send()
        .await
        .expect("the login is submitted");
    let status = response.status();
    let cookie = session_cookie(&response);
    let body = if status.is_redirection() {
        String::new()
    } else {
        response.text().await.unwrap_or_default()
    };

    assert!(
        status.is_redirection(),
        "the login redirects, status was {status}, body was: {body}"
    );
    cookie
        .or(Some(page_cookie))
        .expect("the login starts a session")
}

/// Initializes the log of the tests, when the environment asks for one.
///
/// `RUST_LOG=next_web_security=trace cargo test --test end_to_end_flow --
/// --nocapture` shows what the chain did with every request.
fn init_log() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_login_page_and_the_passkey_script_are_served() {
    let base = serve(http_security).await;
    let client = client();

    let page = client
        .get(format!("{base}/login"))
        .send()
        .await
        .expect("the login page is served");
    assert_eq!(page.status(), 200);
    let html = page.text().await.expect("the page is text");
    assert!(
        html.contains("passkey-signin"),
        "the page offers the passkey sign-in"
    );
    assert!(
        html.contains("/login/webauthn.js"),
        "the page loads the script of the ceremony"
    );

    let script = client
        .get(format!("{base}/login/webauthn.js"))
        .send()
        .await
        .expect("the script is served");
    assert_eq!(script.status(), 200);
    let javascript = script.text().await.expect("the script is text");
    assert!(
        javascript.contains("/webauthn/authenticate/options"),
        "the script talks to the endpoints of the ceremony"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_form_login_signs_the_user_in() {
    init_log();
    let base = serve(http_security).await;
    let client = client();

    let cookie = sign_in_with_password(&client, &base).await;

    let page = client
        .get(format!("{base}/"))
        .header("cookie", cookie)
        .send()
        .await
        .expect("the page is served");
    assert_eq!(page.status(), 200);
    let html = page.text().await.expect("the page is text");
    assert!(
        html.contains(&format!("<p id=\"user\">{USER}</p>")),
        "the application sees the signed-in user, was: {html}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_passkey_is_registered_and_then_signs_the_user_in() {
    init_log();
    let base = serve(http_security).await;
    let origin = base.clone();
    let key = authenticator_key();
    let signed_in = client();
    let cookie = sign_in_with_password(&signed_in, &base).await;

    // The registration page carries the header its submissions must send.
    let page = signed_in
        .get(format!("{base}/webauthn/register"))
        .header("cookie", &cookie)
        .send()
        .await
        .expect("the registration page is served");
    let status = page.status();
    let html = page.text().await.expect("the page is text");
    assert_eq!(
        status,
        200,
        "the registration page is served, page: {}",
        &html[..html.len().min(600)]
    );
    let (header_name, header_value) = csrf_header(&html);

    let options = signed_in
        .post(format!("{base}/webauthn/register/options"))
        .header("cookie", &cookie)
        .header(&header_name, &header_value)
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("the options are asked");
    assert_eq!(options.status(), 200, "the options are answered");
    let options: serde_json::Value = options.json().await.expect("the options are JSON");
    let challenge = options["challenge"]
        .as_str()
        .expect("the options carry a challenge")
        .to_string();
    assert_eq!(options["rp"]["id"], "127.0.0.1");

    let registration = serde_json::json!({
        "id": base64_url(CREDENTIAL_ID),
        "rawId": base64_url(CREDENTIAL_ID),
        "type": "public-key",
        "response": {
            "clientDataJSON": base64_url(&client_data_json("webauthn.create", &challenge, &origin)),
            "attestationObject": base64_url(&attestation_object("127.0.0.1", CREDENTIAL_ID, &key)),
            "transports": ["internal"],
        },
    });
    let registered = signed_in
        .post(format!("{base}/webauthn/register"))
        .header("cookie", &cookie)
        .header(&header_name, &header_value)
        .json(&registration)
        .send()
        .await
        .expect("the registration is submitted");
    assert_eq!(
        registered.status(),
        200,
        "the registration is taken, body was: {}",
        registered.text().await.unwrap_or_default()
    );

    // A passkey login is a new browser: no session, no password.
    let sign_in = client();
    let login_page = sign_in
        .get(format!("{base}/login"))
        .send()
        .await
        .expect("the login page is served");
    let login_cookie = session_cookie(&login_page).expect("the login page starts a session");
    let login_page = login_page.text().await.expect("the page is text");
    let (login_header_name, login_header_value) = csrf_header(&login_page);
    let options = sign_in
        .post(format!("{base}/webauthn/authenticate/options"))
        .header("cookie", &login_cookie)
        .header(&login_header_name, &login_header_value)
        .json(&serde_json::json!({ "username": USER }))
        .send()
        .await
        .expect("the options are asked");
    assert_eq!(options.status(), 200, "the options are answered");
    let options: serde_json::Value = options.json().await.expect("the options are JSON");
    let challenge = options["challenge"]
        .as_str()
        .expect("the options carry a challenge")
        .to_string();
    assert_eq!(
        options["allowCredentials"][0]["id"],
        base64_url(CREDENTIAL_ID),
        "the ceremony offers the credential that was registered"
    );

    let client_data = client_data_json("webauthn.get", &challenge, &origin);
    let authenticator_data = assertion_data("127.0.0.1", 1);
    let mut message = authenticator_data.clone();
    message.extend_from_slice(&sha256(&client_data));
    let assertion = serde_json::json!({
        "id": base64_url(CREDENTIAL_ID),
        "rawId": base64_url(CREDENTIAL_ID),
        "type": "public-key",
        "response": {
            "clientDataJSON": base64_url(&client_data),
            "authenticatorData": base64_url(&authenticator_data),
            "signature": base64_url(&sign(&key, &message)),
            "userHandle": serde_json::Value::Null,
        },
    });
    let response = sign_in
        .post(format!("{base}/webauthn/authenticate"))
        .header("cookie", &login_cookie)
        .header(&login_header_name, &login_header_value)
        .json(&assertion)
        .send()
        .await
        .expect("the assertion is submitted");
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        location, "/",
        "the passkey login redirects to the application, status was {}",
        response.status()
    );
    let cookie = session_cookie(&response).unwrap_or(login_cookie);

    let page = sign_in
        .get(format!("{base}/"))
        .header("cookie", cookie)
        .send()
        .await
        .expect("the page is served");
    let html = page.text().await.expect("the page is text");
    assert!(
        html.contains(&format!("<p id=\"user\">{USER}</p>")),
        "the passkey signed the user in, was: {html}"
    );
}
