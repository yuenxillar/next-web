//! The `v2` security tree in a real Next Web application.
//!
//! The application is built the way a deployment builds it: `NextWebApplication`
//! refreshes the context, resolves the `Arc<dyn HttpFilter>` the application
//! contributes, installs the layer that runs it in front of the routes, and
//! serves the router on a real socket. The test drives that server with a real
//! HTTP client that keeps the cookies of the responses, the way a browser does.
//!
//! The flow is the one an application relies on in production: the login form,
//! the session cookie, the role of the account, the unauthorized page, the
//! logout, the challenge of the HTTP authentication, remember-me, the invalid
//! request filter, and several sessions served at once.

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::{Router, routing::get};
use next_web::{
    Application, ConfigurableApplicationContext, NextWebApplication,
    core::{any::AnyValue, http::filter::HttpFilter, util::indexmap::IndexMap},
    macros::bind::singleton,
};
use next_web_security::{
    core::realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
    web::{
        filter_proxy::FilterProxy, filter_proxy_configure::FilterProxyConfigure,
        mgt::default_web_security_manager::DefaultWebSecurityManager,
    },
};
use reqwest::{Client, StatusCode, redirect::Policy};
use tokio::net::TcpStream;

/// The application of the test: the routes behind the security filter.
#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {
    fn router(&self, _ctx: &mut dyn ConfigurableApplicationContext) -> Router {
        Router::new()
            .route("/", get(|| async { "home" }))
            .route("/home", get(|| async { "home" }))
            .route("/public/hello", get(|| async { "public hello" }))
            .route(
                "/login.jsp",
                get(|| async { "login page" }).post(|| async { "login page" }),
            )
            .route("/logout", get(|| async { "logout page" }))
            .route("/forbidden", get(|| async { "forbidden" }))
            .route("/admin/reports", get(|| async { "admin reports" }))
            .route("/account", get(|| async { "account" }))
            .route("/api/data", get(|| async { "api data" }))
    }
}

/// The security filter the application contributes.
///
/// The layer of the framework resolves `Arc<dyn HttpFilter>` from the
/// application context and runs every filter it finds in front of the routes,
/// so registering the proxy this way is the production wiring.
#[singleton]
fn security_filter() -> Arc<dyn HttpFilter> {
    let mut realm = SimpleAccountRealm::with_name("iniRealm");
    realm.add_account(
        SimpleAccount::new("root")
            .with_password("secret")
            .with_role("admin"),
    );
    realm.add_account(SimpleAccount::new("guest").with_password("guest"));

    let mut chains = IndexMap::new();
    chains.insert("/login.jsp".to_string(), "authc".to_string());
    chains.insert("/logout".to_string(), "logout".to_string());
    chains.insert("/account/**".to_string(), "authc".to_string());
    chains.insert("/admin/**".to_string(), "roles[admin]".to_string());
    chains.insert("/api/**".to_string(), "authcBasic".to_string());
    chains.insert("/**".to_string(), "anon".to_string());

    let mut configure = FilterProxyConfigure::default();
    configure.set_login_url("/login.jsp");
    configure.set_success_url("/home");
    configure.set_unauthorized_url("/forbidden");
    configure.set_filter_chain_definition_map(chains);

    Arc::new(FilterProxy::new(
        DefaultWebSecurityManager::from(realm),
        configure,
    ))
}

/// Returns an address on the loopback interface that is free right now.
fn free_address() -> SocketAddr {
    let listener =
        std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a free port is bound");
    listener.local_addr().expect("the address of the socket")
}

/// Returns a client of the server that keeps its cookies, like a browser.
fn new_browser() -> Client {
    Client::builder()
        .cookie_store(true)
        .redirect(Policy::none())
        .build()
        .expect("the client is built")
}

/// Starts the application of the test on its own runtime, the way a deployment
/// starts it.
///
/// The application is built inside the thread because it is not `Send` — it
/// carries a banner and the initializers of the framework — and the thread owns
/// the runtime the application runs on. The server ends with the test process.
/// The caller waits for the server with [`wait_until_serving`].
fn start_application(address: SocketAddr) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("the runtime of the application is built");

        runtime.block_on(async move {
            let properties = HashMap::from([
                (
                    "next.server.address".to_string(),
                    AnyValue::from(address.ip().to_string()),
                ),
                (
                    "next.server.port".to_string(),
                    AnyValue::from(address.port().to_string()),
                ),
            ]);

            let mut application = NextWebApplication::<TestApplication>::default();
            application.set_default_properties(properties);
            application.set_add_command_line_properties(false);

            application.run().await;
        });
    });
}

/// Waits until the application of the address answers, or gives up.
async fn wait_until_serving(address: SocketAddr) {
    for _ in 0..250 {
        if TcpStream::connect(address).await.is_ok() {
            return;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("the application never served {address}");
}

/// Returns the value of the `Location` header of a response.
fn location(response: &reqwest::Response) -> Option<&str> {
    response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
}

/// Returns the `Set-Cookie` header of the session of a response.
fn session_cookie(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("JSESSIONID="))
        .map(str::to_string)
}

/// Returns the `rememberMe` cookie of a response, when the login asked to be
/// remembered.
fn remember_cookie(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("rememberMe=") && !value.starts_with("rememberMe=deleteMe"))
        .map(str::to_string)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_whole_security_flow_runs_on_a_real_server() {
    let address = free_address();
    start_application(address);
    wait_until_serving(address).await;

    let base = format!("http://{address}");
    let browser = new_browser();

    // An anonymous visitor reads the public path.
    let response = browser
        .get(format!("{base}/public/hello"))
        .send()
        .await
        .expect("the public path is answered");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.text().await.expect("the body is read"),
        "public hello"
    );

    // The global filter of the application refuses a request that cannot be
    // served safely, before the chain of the path answers it.
    let response = browser
        .get(format!("{base}/admin;x"))
        .send()
        .await
        .expect("the invalid request is answered");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // The administrative area is refused to the same visitor: the roles filter
    // answers with the unauthorized page of the application, and the routes are
    // not reached.
    let response = browser
        .get(format!("{base}/admin/reports"))
        .send()
        .await
        .expect("the administrative path is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/forbidden"));

    // The login page itself is served to an anonymous visitor.
    let response = browser
        .get(format!("{base}/login.jsp"))
        .send()
        .await
        .expect("the login page is answered");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.text().await.expect("the body is read"),
        "login page"
    );

    // The form login: the layer buffers the form body, the filter builds the
    // token from it, authenticates it, opens the session and returns the
    // `Set-Cookie` of it, and the chain does not continue to the routes.
    let response = browser
        .post(format!("{base}/login.jsp"))
        .form(&[("username", "root"), ("password", "secret")])
        .send()
        .await
        .expect("the login is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/home"));
    let cookie = session_cookie(&response).expect("the login writes the session cookie");
    assert!(
        cookie.contains("Path=/"),
        "the session cookie belongs to every path: {cookie}"
    );

    // The next request presents the session, and the roles filter reads the
    // account of the login and lets it through.
    let response = browser
        .get(format!("{base}/admin/reports"))
        .send()
        .await
        .expect("the administrative path is answered");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.text().await.expect("the body is read"),
        "admin reports"
    );

    // The same proxy serves several requests of different clients at once.
    let mut requests = Vec::new();
    for _ in 0..16 {
        let client = browser.clone();
        let url = format!("{base}/admin/reports");
        requests.push(tokio::spawn(async move {
            client
                .get(url)
                .send()
                .await
                .map(|response| response.status())
        }));
    }
    for request in requests {
        assert_eq!(
            request
                .await
                .expect("the request is answered")
                .expect("the connection holds"),
            StatusCode::OK
        );
    }

    // The logout filter ends the session of the account and sends the client
    // home; the same client is anonymous again afterwards.
    let response = browser
        .get(format!("{base}/logout"))
        .send()
        .await
        .expect("the logout is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/"));

    let response = browser
        .get(format!("{base}/admin/reports"))
        .send()
        .await
        .expect("the administrative path is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/forbidden"));

    // A logged in account without the role is still refused: `guest` is
    // authenticated, and the chain of `/admin/**` requires `admin`.
    let guest = new_browser();
    let response = guest
        .post(format!("{base}/login.jsp"))
        .form(&[("username", "guest"), ("password", "guest")])
        .send()
        .await
        .expect("the login is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/home"));

    let response = guest
        .get(format!("{base}/admin/reports"))
        .send()
        .await
        .expect("the administrative path is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/forbidden"));

    // Two sessions at once: the application serves two accounts concurrently,
    // and every request is allowed by the session it presents.
    let other_root = new_browser();
    let response = other_root
        .post(format!("{base}/login.jsp"))
        .form(&[("username", "root"), ("password", "secret")])
        .send()
        .await
        .expect("the login is answered");
    assert_eq!(response.status(), StatusCode::FOUND);

    let mut admin_requests = Vec::new();
    let mut guest_requests = Vec::new();
    for _ in 0..8 {
        let client = other_root.clone();
        let url = format!("{base}/admin/reports");
        admin_requests.push(tokio::spawn(async move {
            client
                .get(url)
                .send()
                .await
                .map(|response| response.status())
        }));

        let client = guest.clone();
        let url = format!("{base}/admin/reports");
        guest_requests.push(tokio::spawn(async move {
            client
                .get(url)
                .send()
                .await
                .map(|response| response.status())
        }));
    }

    for request in admin_requests {
        assert_eq!(
            request
                .await
                .expect("the request is answered")
                .expect("the connection holds"),
            StatusCode::OK
        );
    }

    for request in guest_requests {
        assert_eq!(
            request
                .await
                .expect("the request is answered")
                .expect("the connection holds"),
            StatusCode::FOUND
        );
    }

    // A wrong password answers the login page again, and the client stays
    // anonymous.
    let stranger = new_browser();
    let response = stranger
        .post(format!("{base}/login.jsp"))
        .form(&[("username", "root"), ("password", "wrong")])
        .send()
        .await
        .expect("the login is answered");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        session_cookie(&response).is_none(),
        "a refused login opens no session"
    );
    assert_eq!(
        response.text().await.expect("the body is read"),
        "login page"
    );

    let response = stranger
        .get(format!("{base}/admin/reports"))
        .send()
        .await
        .expect("the administrative path is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/forbidden"));

    // The HTTP basic chain challenges an anonymous request and accepts the
    // credentials of the account.
    let response = browser
        .get(format!("{base}/api/data"))
        .send()
        .await
        .expect("the API is answered");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        response.headers().get("www-authenticate").is_some(),
        "the challenge names the scheme of the filter"
    );

    let response = browser
        .get(format!("{base}/api/data"))
        .basic_auth("root", Some("secret"))
        .send()
        .await
        .expect("the API is answered");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.expect("the body is read"), "api data");

    // A login that asks to be remembered leaves an identity cookie. A client
    // that presents only that cookie is remembered: the roles filter reads the
    // account from it, while the `authc` chain still asks for a login, because
    // a remembered subject is not an authenticated one.
    let remembered_browser = new_browser();
    let response = remembered_browser
        .post(format!("{base}/login.jsp"))
        .form(&[
            ("username", "root"),
            ("password", "secret"),
            ("rememberMe", "true"),
        ])
        .send()
        .await
        .expect("the login is answered");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/home"));
    let remember = remember_cookie(&response).expect("the login remembers the identity");
    let remember_pair = remember
        .split(';')
        .next()
        .expect("the cookie carries a value");

    let remembered = Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("the client is built");

    let response = remembered
        .get(format!("{base}/admin/reports"))
        .header("cookie", remember_pair)
        .send()
        .await
        .expect("the administrative path is answered");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "the remembered identity holds the role of the account"
    );

    let response = remembered
        .get(format!("{base}/account"))
        .header("cookie", remember_pair)
        .send()
        .await
        .expect("the account path is answered");
    assert_eq!(
        response.status(),
        StatusCode::FOUND,
        "a remembered subject still has to log in on the `authc` chain"
    );
    assert_eq!(location(&response), Some("/login.jsp"));
}
